use near_sdk::{near, env, require, Timestamp, AccountId, EpochHeight};
use std::collections::HashMap;
use near_sdk::json_types::U128;

type Balance = u128;

/// Voting contract for any specific proposal. Once the majority of the stake holders agree to
/// the proposal, the time will be recorded and the voting ends.
#[near(contract_state)]
#[derive(Default)]
pub struct Contract {
    proposal: String,
    deadline_timestamp_ms: Timestamp,
    votes: HashMap<AccountId, Balance>,
    total_voted_stake: Balance,
    result: Option<Timestamp>,
    last_epoch_height: EpochHeight,
}

// Implement the contract structure
#[near]
impl Contract {
    #[init]
    #[private]
    pub fn new(proposal: String, deadline_timestamp_ms: Timestamp) -> Self {
        require!(!proposal.is_empty(), "Proposal cannot be empty");
        require!(deadline_timestamp_ms > env::block_timestamp_ms(), "Deadline must be in the future");
        Contract {
            proposal,
            deadline_timestamp_ms,
            votes: HashMap::new(),
            total_voted_stake: 0,
            result: None,
            last_epoch_height: 0,
        }
    }

    /// Ping to update the votes according to current stake of validators.
    pub fn ping(&mut self) {
        require!(self.result.is_none(), "Voting has already ended");
        let cur_epoch_height = env::epoch_height();
        if cur_epoch_height != self.last_epoch_height {
            let votes = std::mem::take(&mut self.votes);
            self.total_voted_stake = 0;
            for (account_id, _) in votes {
                let account_current_stake = env::validator_stake(&account_id).as_yoctonear();
                self.total_voted_stake += account_current_stake;
                if account_current_stake > 0 {
                    self.votes.insert(account_id, account_current_stake);
                }
            }
            self.check_result();
            self.last_epoch_height = cur_epoch_height;
        }
    }

    /// Check whether the voting has ended.
    fn check_result(&mut self) {
        require!(
            self.result.is_none(),
            "check result is called after result is already set"
        );
        let total_stake = env::validator_total_stake().as_yoctonear();
        if self.total_voted_stake > total_stake * 2 / 3 {
            self.result = Some(env::block_timestamp_ms());
        }
    }

    /// Method for validators to vote or withdraw the vote.
    /// Votes for if `is_vote` is true, or withdraws the vote if `is_vote` is false.
    pub fn vote(&mut self, is_vote: bool) {
        require!(
            env::block_timestamp_ms() < self.deadline_timestamp_ms,
            "Voting deadline has already passed"
        );
        require!(self.result.is_none(), "Voting has already completed");

        self.ping();
        let account_id = env::predecessor_account_id();
        let account_stake = if is_vote {
            let stake = env::validator_stake(&account_id).as_yoctonear();
            require!(stake > 0, format!("{} is not a validator", account_id));
            stake
        } else {
            0
        };
        let voted_stake = self.votes.remove(&account_id).unwrap_or_default();
        require!(
            voted_stake <= self.total_voted_stake,
            format!("invariant: voted stake {} is more than total voted stake {}", voted_stake, self.total_voted_stake)
        );
        self.total_voted_stake = self.total_voted_stake + account_stake - voted_stake;
        if account_stake > 0 {
            self.votes.insert(account_id, account_stake);
            self.check_result();
        }
    }

    /// Returns current a pair of `total_voted_stake` and the total stake.
    /// Note: as a view method, it doesn't recompute the active stake. May need to call `ping` to
    /// update the active stake.
    pub fn get_total_voted_stake(&self) -> (U128, U128) {
        (
            self.total_voted_stake.into(),
            env::validator_total_stake().as_yoctonear().into(),
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


#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::{env, testing_env, test_vm_config, RuntimeFeesConfig, NearToken, Gas};

    fn validators() -> HashMap<String, NearToken> {
        (0..10).map(|i| (format!("validator-{}", i), NearToken::from_yoctonear(10))).collect::<HashMap<_, _>>()
    }

    fn validator(id: u64) -> AccountId {
        format!("validator-{}", id).parse().unwrap()
    }

    fn get_context(predecessor_account_id: &AccountId) -> VMContextBuilder {
        get_context_with_epoch_height(predecessor_account_id, 0)
    }

    fn get_context_with_epoch_height(
        predecessor_account_id: &AccountId,
        epoch_height: EpochHeight,
    ) -> VMContextBuilder {
        VMContextBuilder::new()
            .current_account_id(accounts(0).clone())
            .signer_account_id(accounts(1).clone())
            // .signer_account_pk(vec![0, 1, 2])
            .predecessor_account_id(predecessor_account_id.clone())
            // .block_index(0)
            // .block_timestamp(0)
            // .account_balance(0)
            // .account_locked_balance(0)
            .storage_usage(1000)
            // .attached_deposit(0)
            .prepaid_gas(Gas::from_tgas(200))
            // .random_seed(vec![0, 1, 2])
            .is_view(false)
            // .output_data_receivers(vec![])
            .epoch_height(epoch_height)
            .clone()
    }

    fn setup() -> (Contract, VMContextBuilder) {
        let context = VMContextBuilder::new();

        let contract = Contract::new(
            "Test proposal".to_string(),
            env::block_timestamp_ms() + 1000, // 1 second deadline
        );
        
        (contract, context)
    }

    #[test]
    #[should_panic(expected = "is not a validator")]
    fn test_nonvalidator_cannot_vote() {
        let context = get_context(&validator(3));
        let validators = HashMap::from_iter(
            vec![
                (validator(0).to_string(), NearToken::from_yoctonear(100)),
                (validator(1).to_string(), NearToken::from_yoctonear(100)),
            ]
            .into_iter(),
        );
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators);
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);
        contract.vote(true);
    }

    #[test]
    #[should_panic(expected = "Voting has already completed")]
    fn test_vote_again_after_voting_ends() {
        // Setup validator and context
        let validator_id = validator(0);
        let context = get_context(&validator_id);
        let validators = HashMap::from_iter(vec![(validator_id.to_string(), NearToken::from_yoctonear(100))].into_iter());
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators);
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);
        contract.vote(true);
        assert!(contract.get_result().is_some());
        contract.vote(true); // Should panic because voting has ended
    }

    #[test]
    fn test_voting_simple() {
        let validators: HashMap<String, NearToken> = (0..10)
            .map(|i| (format!("validator-{}", i), NearToken::from_yoctonear(10)))
            .collect();
        let context = get_context(&validator(0));
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);

        for i in 0..7 {
            let voter = validator(i);
            let context = get_context(&voter);
            testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
            contract.vote(true);
            // Simulate view context (not strictly necessary for this contract, but for parity with original)
            // let mut context = get_context(&voter);
            // context.is_view = true;
            // testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
            assert_eq!(
                contract.get_total_voted_stake(),
                (U128::from(10 * (i + 1) as u128), U128::from(100))
            );
            let expected_votes: HashMap<AccountId, U128> = (0..=i)
                .map(|j| (validator(j), U128::from(10)))
                .collect();
            assert_eq!(contract.get_votes(), expected_votes);
            assert_eq!(contract.get_votes().len() as u64, i + 1);
            if i < 6 {
                assert!(contract.get_result().is_none());
            } else {
                assert!(contract.get_result().is_some());
            }
        }
    }

    #[test]
    fn test_voting_with_epoch_change() {
        let validators: HashMap<String, NearToken> = (0..10)
            .map(|i| (format!("validator-{}", i), NearToken::from_yoctonear(10)))
            .collect();
        let context = get_context(&validator(0));
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);

        for i in 0..7 {
            let voter = validator(i);
            let context = get_context_with_epoch_height(&voter, i);
            testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
            contract.vote(true);
            assert_eq!(contract.get_votes().len() as u64, i + 1);
            if i < 6 {
                assert!(contract.get_result().is_none());
            } else {
                assert!(contract.get_result().is_some());
            }
        }
    }

    #[test]
    fn test_validator_stake_change() {
        let mut validators: HashMap<String, NearToken> = HashMap::from_iter(vec![
            (validator(1).to_string(), NearToken::from_yoctonear(40)),
            (validator(2).to_string(), NearToken::from_yoctonear(10)),
            (validator(3).to_string(), NearToken::from_yoctonear(10)),
        ]);
        let context = get_context_with_epoch_height(&validator(1), 1);
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);
        contract.vote(true);
        validators.insert(validator(1).to_string(), NearToken::from_yoctonear(50));
        let context = get_context_with_epoch_height(&validator(2), 2);
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        contract.ping();
        assert!(contract.get_result().is_some());
    }

    #[test]
    fn test_withdraw_votes() {
        let validators: HashMap<String, NearToken> = HashMap::from_iter(vec![
            (validator(1).to_string(), NearToken::from_yoctonear(10)),
            (validator(2).to_string(), NearToken::from_yoctonear(10)),
        ]);
        let context = get_context_with_epoch_height(&validator(1), 1);
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);
        contract.vote(true);
        assert_eq!(contract.get_votes().len(), 1);
        let context = get_context_with_epoch_height(&validator(1), 2);
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        contract.vote(false);
        assert!(contract.get_votes().is_empty());
    }

    #[test]
    fn test_validator_kick_out() {
        let mut validators: HashMap<String, NearToken> = HashMap::from_iter(vec![
            (validator(1).to_string(), NearToken::from_yoctonear(40)),
            (validator(2).to_string(), NearToken::from_yoctonear(10)),
            (validator(3).to_string(), NearToken::from_yoctonear(10)),
        ]);
        let context = get_context_with_epoch_height(&validator(1), 1);
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        let mut contract = Contract::new("Test proposal".to_string(), env::block_timestamp_ms() + 1000);
        contract.vote(true);
        assert_eq!((contract.get_total_voted_stake().0).0, 40);
        validators.remove(&validator(1).to_string());
        let context = get_context_with_epoch_height(&validator(2), 2);
        testing_env!(context.build(), test_vm_config(), RuntimeFeesConfig::test(), validators.clone());
        contract.ping();
        assert_eq!((contract.get_total_voted_stake().0).0, 0);
    }

    #[test]
    fn test_get_proposal() {
        let (contract, _) = setup();
        assert_eq!(contract.get_proposal(), "Test proposal");
    }

    #[test]
    fn test_get_deadline_timestamp() {
        let (contract, _) = setup();
        assert_eq!(contract.get_deadline_timestamp(), env::block_timestamp_ms() + 1000);
    }

    #[test]
    #[should_panic(expected = "Proposal cannot be empty")]
    fn test_empty_proposal() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());
        Contract::new("".to_string(), env::block_timestamp_ms() + 1000);
    }

    #[test]
    #[should_panic(expected = "Deadline must be in the future")]
    fn test_past_deadline() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());
        Contract::new("Test proposal".to_string(), env::block_timestamp_ms());
    }

    #[test]
    #[should_panic(expected = "Voting deadline has already passed")]
    fn test_vote_after_deadline() {
        let (mut contract, mut context) = setup();

        // Move time past deadline
        testing_env!(context
            .block_timestamp(env::block_timestamp_ms() + 2000 * 1_000_000)
            .predecessor_account_id(validator(0))
            .build(),
            test_vm_config(),
            RuntimeFeesConfig::test(),
            validators()
        );
        
        contract.vote(true);
    }
}
