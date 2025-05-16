use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, ext_contract, log, near, require, AccountId, BorshStorageKey, Gas, NearToken,
    PanicOnDefault, Promise, PromiseError, PublicKey,
};

type Balance = u128;

const VOTE_GAS: Gas = Gas::from_tgas(100);
const STAKE_CALLBACK_GAS: Gas = Gas::from_tgas(5);

#[ext_contract(ext_voting)]
trait VotingContract {
    fn vote(&mut self, is_vote: bool);
}

#[near]
#[derive(BorshStorageKey)]
pub enum Prefix {
    Accounts,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
struct MockStakingPool {
    owner_id: AccountId,
    stake_public_key: PublicKey,
    accounts: LookupMap<AccountId, Balance>,
    total_staked_balance: Balance,
}

#[near]
impl MockStakingPool {
    #[init]
    #[private]
    pub fn new(owner_id: AccountId, stake_public_key: PublicKey) -> Self {
        Self {
            owner_id,
            stake_public_key,
            accounts: LookupMap::new(Prefix::Accounts),
            total_staked_balance: 0,
        }
    }

    #[payable]
    pub fn deposit_and_stake(&mut self) -> Promise {
        let amount = env::attached_deposit().as_yoctonear();
        require!(amount > 0u128, "Invalid stake amount");

        let account_id = env::predecessor_account_id();
        let balance = self.internal_account_staked_balance(&account_id);
        self.accounts.insert(account_id, balance + amount);
        self.total_staked_balance += amount;

        self.internal_restake()
    }

    pub fn unstake(&mut self, amount: U128) -> Promise {
        let amount = amount.0;
        require!(amount > 0u128, "Invalid unstake amount");

        let account_id = env::predecessor_account_id();
        let balance = self.internal_account_staked_balance(&account_id);
        require!(balance >= amount, "Not enough stake");

        self.accounts.insert(account_id, balance - amount);
        self.total_staked_balance -= amount;

        self.internal_restake()
    }

    /// Owner's method.
    /// Calls `vote(is_vote)` on the given voting contract account ID on behalf of the pool.
    pub fn vote(&mut self, voting_account_id: AccountId, is_vote: bool) -> Promise {
        self.assert_owner();
        ext_voting::ext(voting_account_id)
            .with_static_gas(VOTE_GAS)
            .vote(is_vote)
    }

    pub fn get_staked_balance(&self) -> (U128, U128) {
        (
            U128::from(env::validator_stake(&env::current_account_id()).as_yoctonear()),
            U128::from(self.total_staked_balance),
        )
    }

    #[private]
    pub fn on_stake_action(&self, #[callback_result] result: Result<String, PromiseError>) {
        if result.is_err() {
            log!("Stake action failed");
            return;
        }

        log!(
            "Validator stake amount: {}",
            env::validator_stake(&env::current_account_id())
        );
    }

    fn internal_account_staked_balance(&self, account_id: &AccountId) -> Balance {
        *self.accounts.get(account_id).unwrap_or(&0u128)
    }

    fn internal_restake(&self) -> Promise {
        Promise::new(env::current_account_id())
            .stake(
                NearToken::from_yoctonear(self.total_staked_balance),
                self.stake_public_key.clone(),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(STAKE_CALLBACK_GAS)
                    .on_stake_action(),
            )
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner_id, "Not owner");
    }
}
