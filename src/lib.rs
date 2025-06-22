mod events;
mod utils;

use events::Event;
use near_sdk::json_types::{U128, U64};
use near_sdk::{
    env, ext_contract, near, require, AccountId, EpochHeight, Gas, PanicOnDefault, Promise,
    PromiseError,
};
use std::collections::HashMap;
use utils::{validator_stake, validator_total_stake};

/// Balance in yocto NEAR
type Balance = u128;
/// Timestamp in milliseconds
type Timestamp = u64;

const GET_OWNER_ID_GAS: Gas = Gas::from_tgas(5);

/// Voting contract for any specific proposal. Once the majority of the stake holders agree to
/// the proposal, the time will be recorded and the voting ends.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    proposal: String,
    deadline_timestamp_ms: Timestamp,
    votes: HashMap<AccountId, Balance>,
    total_voted_stake: Balance,
    result: Option<Timestamp>,
    last_epoch_height: EpochHeight,
}

#[ext_contract(ext_staking_pool)]
pub trait StakingPoolContract {
    fn get_owner_id(&self) -> AccountId;
}

// Implement the contract structure
#[near]
impl Contract {
    #[init]
    #[private]
    pub fn new(proposal: String, deadline_timestamp_ms: Timestamp) -> Self {
        require!(!proposal.is_empty(), "Proposal cannot be empty");
        require!(
            deadline_timestamp_ms > env::block_timestamp_ms(),
            "Deadline must be in the future"
        );
        Self {
            proposal,
            deadline_timestamp_ms,
            votes: HashMap::new(),
            total_voted_stake: 0,
            result: None,
            last_epoch_height: 0,
        }
    }

    /// Method for validators to vote or withdraw the vote.
    /// Votes for if `is_vote` is true, or withdraws the vote if `is_vote` is false.
    pub fn vote(&mut self, is_vote: bool, staking_pool_id: AccountId) -> Promise {
        ext_staking_pool::ext(staking_pool_id.clone())
            .with_static_gas(GET_OWNER_ID_GAS)
            .get_owner_id()
            .then(Self::ext(env::current_account_id()).on_get_owner_id(
                env::predecessor_account_id(),
                staking_pool_id,
                is_vote,
            ))
    }

    /// Ping to update the votes according to current stake of validators.
    pub fn ping(&mut self) {
        require!(
            env::block_timestamp_ms() < self.deadline_timestamp_ms,
            "Voting deadline has already passed"
        );
        require!(self.result.is_none(), "Voting has already ended");
        let cur_epoch_height = env::epoch_height();
        if cur_epoch_height != self.last_epoch_height {
            let votes = std::mem::take(&mut self.votes);
            self.total_voted_stake = 0;
            for (account_id, _) in votes {
                let account_current_stake = validator_stake(&account_id);
                self.total_voted_stake += account_current_stake;
                if account_current_stake > 0 {
                    self.votes.insert(account_id, account_current_stake);
                }
            }
            self.check_result();
            self.last_epoch_height = cur_epoch_height;
        }
    }

    /// Check the owner id and vote.
    #[private]
    pub fn on_get_owner_id(
        &mut self,
        pool_owner_id: AccountId,
        staking_pool_id: AccountId,
        is_vote: bool,
        #[callback_result] pool_owner_id_result: Result<AccountId, PromiseError>,
    ) {
        if let Ok(actual_owner_id) = pool_owner_id_result {
            require!(
                pool_owner_id == actual_owner_id,
                "Voting is only allowed for the staking pool owner"
            );
            self.internal_vote(is_vote, staking_pool_id);
        } else {
            env::panic_str("Failed to get the staking pool owner id");
        }
    }

    /// Internal method for voting.
    fn internal_vote(&mut self, is_vote: bool, account_id: AccountId) {
        self.ping();

        let account_stake = if is_vote {
            let stake = validator_stake(&account_id);
            require!(stake > 0, format!("{} is not a validator", account_id));
            stake
        } else {
            0
        };
        let voted_stake = self.votes.remove(&account_id).unwrap_or_default();
        require!(
            voted_stake <= self.total_voted_stake,
            format!(
                "invariant: voted stake {} is more than total voted stake {}",
                voted_stake, self.total_voted_stake
            )
        );
        self.total_voted_stake = self.total_voted_stake + account_stake - voted_stake;
        if account_stake > 0 {
            self.votes.insert(account_id.clone(), account_stake);
            self.check_result();
        }
        // emit event
        if is_vote {
            Event::Voted {
                validator_id: &account_id,
            }
            .emit();
        } else {
            Event::VoteWithdrawn {
                validator_id: &account_id,
            }
            .emit();
        }
    }

    /// Check whether the voting has ended.
    fn check_result(&mut self) {
        require!(
            self.result.is_none(),
            "check result is called after result is already set"
        );
        let total_stake = validator_total_stake();
        if self.total_voted_stake > total_stake * 2 / 3 {
            self.result = Some(env::block_timestamp_ms());
            Event::ProposalApproved {
                proposal: &self.proposal,
                approval_timestamp_ms: &U64::from(env::block_timestamp_ms()),
                deadline_timestamp_ms: &U64::from(self.deadline_timestamp_ms),
                voted_stake: &U128::from(self.total_voted_stake),
                total_stake: &U128::from(total_stake),
                num_votes: &U64::from(self.votes.len() as u64),
            }
            .emit();
        }
    }
}

/// View methods
#[near]
impl Contract {
    /// Returns a pair of `total_voted_stake` and the total stake.
    /// Note: as a view method, it doesn't recompute the active stake. May need to call `ping` to
    /// update the active stake.
    pub fn get_total_voted_stake(&self) -> (U128, U128) {
        (
            self.total_voted_stake.into(),
            validator_total_stake().into(),
        )
    }

    /// Returns all active votes.
    /// Note: as a view method, it doesn't recompute the active stake. May need to call `ping` to
    /// update the active stake.
    pub fn get_votes(&self) -> HashMap<AccountId, U128> {
        self.votes
            .iter()
            .map(|(account_id, stake)| (account_id.clone(), (*stake).into()))
            .collect()
    }

    /// Get the timestamp of when the voting finishes. `None` means the voting hasn't ended yet.
    pub fn get_result(&self) -> Option<Timestamp> {
        self.result
    }

    /// Returns the deadline timestamp in milliseconds.
    pub fn get_deadline_timestamp(&self) -> Timestamp {
        self.deadline_timestamp_ms
    }

    /// Returns the proposal.
    pub fn get_proposal(&self) -> String {
        self.proposal.clone()
    }
}

#[cfg(feature = "test")]
#[near]
impl Contract {
    pub fn set_validator_stake(&mut self, validator_account_id: AccountId, amount: U128) {
        utils::set_validator_stake(validator_account_id, amount.0)
    }

    pub fn get_validator_stake(&self, validator_account_id: AccountId) -> U128 {
        utils::get_validator_stake(&validator_account_id).into()
    }

    pub fn get_validator_total_stake(&self) -> U128 {
        validator_total_stake().into()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{env, test_vm_config, testing_env, Gas, NearToken, RuntimeFeesConfig};

    fn validators() -> HashMap<String, NearToken> {
        (0..300)
            .map(|i| (format!("validator-{}", i), NearToken::from_yoctonear(10)))
            .collect::<HashMap<_, _>>()
    }

    fn validator(id: u64) -> AccountId {
        format!("validator-{}", id).parse().unwrap()
    }

    fn get_contract() -> Contract {
        Contract::new(
            "Test proposal".to_string(),
            env::block_timestamp_ms() + 1000,
        )
    }

    fn get_context(predecessor_account_id: &AccountId) -> VMContextBuilder {
        get_context_with_epoch_height(predecessor_account_id, 0)
    }

    fn get_context_with_epoch_height(
        predecessor_account_id: &AccountId,
        epoch_height: EpochHeight,
    ) -> VMContextBuilder {
        VMContextBuilder::new()
            .current_account_id(accounts(0))
            .signer_account_id(accounts(1))
            .predecessor_account_id(predecessor_account_id.clone())
            .storage_usage(1000)
            .prepaid_gas(Gas::from_tgas(200))
            .is_view(false)
            .epoch_height(epoch_height)
            .clone()
    }

    fn set_context(context: &VMContextBuilder) {
        testing_env!(
            context.build(),
            test_vm_config(),
            RuntimeFeesConfig::test(),
            validators()
        );
    }

    fn set_context_and_validators(
        context: &VMContextBuilder,
        validators: &HashMap<String, NearToken>,
    ) {
        testing_env!(
            context.build(),
            test_vm_config(),
            RuntimeFeesConfig::test(),
            validators.clone()
        );
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    #[should_panic(expected = "is not a validator")]
    fn test_non_validator_cannot_vote() {
        let context = get_context(&validator(3));
        let validators = HashMap::from_iter(vec![
            (validator(0).to_string(), NearToken::from_yoctonear(100)),
            (validator(1).to_string(), NearToken::from_yoctonear(100)),
        ]);
        set_context_and_validators(&context, &validators);
        let mut contract = get_contract();
        contract.vote(true, validator(0));
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    #[should_panic(expected = "Voting has already ended")]
    fn test_vote_again_after_voting_ends() {
        let validator_id = validator(0);
        let context = get_context(&validator_id);
        let validators = HashMap::from_iter(vec![(
            validator_id.to_string(),
            NearToken::from_yoctonear(100),
        )]);
        set_context_and_validators(&context, &validators);
        let mut contract = get_contract();
        // vote
        contract.vote(true, validator(0));
        assert!(contract.get_result().is_some());
        // vote again. should panic because voting has ended
        contract.vote(true, validator(0));
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    fn test_voting_simple() {
        let context = get_context(&validator(0));
        set_context(&context);
        let mut contract = get_contract();

        for i in 0..201 {
            // vote by each validator
            let voter = validator(i);
            let mut context = get_context(&voter);
            set_context(&context);
            contract.vote(true, validator(0));

            // check total voted stake
            context.is_view(true);
            set_context(&context);
            assert_eq!(
                contract.get_total_voted_stake(),
                (U128::from(10 * (i + 1) as u128), U128::from(3000))
            );
            // check votes
            let expected_votes: HashMap<AccountId, U128> =
                (0..=i).map(|j| (validator(j), U128::from(10))).collect();
            assert_eq!(contract.get_votes(), expected_votes);
            assert_eq!(contract.get_votes().len() as u64, i + 1);
            // check voting result
            if i < 200 {
                assert!(contract.get_result().is_none());
            } else {
                assert!(contract.get_result().is_some());
            }
        }
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    fn test_voting_with_epoch_change() {
        let context = get_context(&validator(0));
        set_context(&context);
        let mut contract = get_contract();

        for i in 0..201 {
            // vote by each validator
            let context = get_context_with_epoch_height(&validator(i), i);
            set_context(&context);
            contract.vote(true, validator(0));
            // check votes
            assert_eq!(contract.get_votes().len() as u64, i + 1);
            // check voting result
            if i < 200 {
                assert!(contract.get_result().is_none());
            } else {
                assert!(contract.get_result().is_some());
            }
        }
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    fn test_validator_stake_change() {
        let mut validators: HashMap<String, NearToken> = HashMap::from_iter(vec![
            (validator(1).to_string(), NearToken::from_yoctonear(40)),
            (validator(2).to_string(), NearToken::from_yoctonear(10)),
            (validator(3).to_string(), NearToken::from_yoctonear(10)),
        ]);
        // vote at epoch 1
        let context = get_context_with_epoch_height(&validator(1), 1);
        set_context_and_validators(&context, &validators);
        let mut contract = get_contract();
        contract.vote(true, validator(0));
        // ping at epoch 2
        validators.insert(validator(1).to_string(), NearToken::from_yoctonear(50));
        let context = get_context_with_epoch_height(&validator(2), 2);
        set_context_and_validators(&context, &validators);
        contract.ping();
        assert!(contract.get_result().is_some());
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    fn test_withdraw_votes() {
        let validators: HashMap<String, NearToken> = HashMap::from_iter(vec![
            (validator(1).to_string(), NearToken::from_yoctonear(10)),
            (validator(2).to_string(), NearToken::from_yoctonear(10)),
        ]);
        let context = get_context_with_epoch_height(&validator(1), 1);
        set_context_and_validators(&context, &validators);
        let mut contract = get_contract();
        // vote at epoch 1
        contract.vote(true, validator(0));
        assert_eq!(contract.get_votes().len(), 1);
        // withdraw vote at epoch 2
        let context = get_context_with_epoch_height(&validator(1), 2);
        set_context_and_validators(&context, &validators);
        contract.vote(false, validator(0));
        assert!(contract.get_votes().is_empty());
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    fn test_validator_kick_out() {
        let mut validators: HashMap<String, NearToken> = HashMap::from_iter(vec![
            (validator(1).to_string(), NearToken::from_yoctonear(40)),
            (validator(2).to_string(), NearToken::from_yoctonear(10)),
            (validator(3).to_string(), NearToken::from_yoctonear(10)),
        ]);
        let context = get_context_with_epoch_height(&validator(1), 1);
        set_context_and_validators(&context, &validators);
        let mut contract = get_contract();
        // vote at epoch 1
        contract.vote(true, validator(0));
        assert_eq!((contract.get_total_voted_stake().0).0, 40);
        assert_eq!(contract.get_votes().len(), 1);
        // remove validator at epoch 2
        validators.remove(&validator(1).to_string());
        let context = get_context_with_epoch_height(&validator(2), 2);
        set_context_and_validators(&context, &validators);
        // ping will update total voted stake
        contract.ping();
        assert_eq!((contract.get_total_voted_stake().0).0, 0);
        assert_eq!(contract.get_votes().len(), 0);
    }

    #[test]
    fn test_init_contract() {
        let contract = get_contract();
        assert_eq!(contract.get_proposal(), "Test proposal");
        assert_eq!(
            contract.get_deadline_timestamp(),
            env::block_timestamp_ms() + 1000
        );
    }

    #[test]
    #[should_panic(expected = "Proposal cannot be empty")]
    fn test_init_with_empty_proposal() {
        let context = VMContextBuilder::new();
        set_context(&context);
        Contract::new("".to_string(), env::block_timestamp_ms() + 1000);
    }

    #[test]
    #[should_panic(expected = "Deadline must be in the future")]
    fn test_init_with_past_deadline() {
        let context = VMContextBuilder::new();
        set_context(&context);
        Contract::new("Test proposal".to_string(), env::block_timestamp_ms());
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    #[should_panic(expected = "Voting deadline has already passed")]
    fn test_vote_after_deadline() {
        let mut contract = get_contract();
        let mut context = get_context(&validator(0));

        // vote after deadline
        set_context(context.block_timestamp(env::block_timestamp_ms() + 2000 * 1_000_000));
        contract.vote(true, validator(0));
    }

    #[ignore = "need to update the test for cross-contract call"]
    #[test]
    #[should_panic(expected = "Voting deadline has already passed")]
    fn test_ping_after_deadline() {
        let mut contract = get_contract();
        let mut context = get_context(&validator(0));

        // vote at epoch 1
        set_context(&context);
        contract.vote(true, validator(0));

        // ping at epoch 2 after deadline
        set_context(
            context
                .block_timestamp(env::block_timestamp_ms() + 2000 * 1_000_000)
                .epoch_height(2),
        );
        contract.ping();
    }
}
