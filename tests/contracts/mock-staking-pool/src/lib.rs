use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, ext_contract, near, require, AccountId, BorshStorageKey, Gas, NearToken,
    PanicOnDefault, Promise, PublicKey,
};

type Balance = u128;

const VOTE_GAS: Gas = Gas::from_tgas(100);

#[allow(dead_code)]
#[ext_contract(ext_voting)]
trait VotingContract {
    fn vote(&mut self, is_vote: bool);

    fn set_validator_stake(validator_account_id: AccountId, amount: U128);
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
    voting_account_id: AccountId,
}

#[near]
impl MockStakingPool {
    #[init]
    #[private]
    pub fn new(owner_id: AccountId, stake_public_key: PublicKey, voting_account_id: AccountId) -> Self {
        Self {
            owner_id,
            stake_public_key,
            accounts: LookupMap::new(Prefix::Accounts),
            total_staked_balance: 0,
            voting_account_id,
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

        self.accounts.insert(account_id.clone(), balance - amount);
        self.total_staked_balance -= amount;

        Promise::new(account_id).transfer(NearToken::from_yoctonear(amount));

        self.internal_restake()
    }

    /// Owner's method.
    /// Calls `vote(is_vote)` on the given voting contract account ID on behalf of the pool.
    pub fn vote(&mut self, voting_account_id: AccountId, is_vote: bool) -> Promise {
        require!(self.voting_account_id == voting_account_id, "Voting account id mismatching");
        self.assert_owner();
        ext_voting::ext(voting_account_id)
            .with_static_gas(VOTE_GAS)
            .vote(is_vote)
    }

    /// Owner's method.
    /// Update voting account ID
    pub fn set_voting_account_id(&mut self, voting_account_id: AccountId) -> Promise {
        self.assert_owner();
        self.voting_account_id = voting_account_id;
        self.internal_restake()
    }

    /// Returns the default voting account ID
    pub fn get_voting_account_id(&self) -> AccountId {
        self.voting_account_id.clone()
    }

    /// Returns the total staking balance.
    pub fn get_total_staked_balance(&self) -> U128 {
        self.total_staked_balance.into()
    }

    pub fn get_owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    fn internal_account_staked_balance(&self, account_id: &AccountId) -> Balance {
        *self.accounts.get(account_id).unwrap_or(&0u128)
    }

    /// Sync stake amount to voting contract
    fn internal_restake(&self) -> Promise {
        ext_voting::ext(self.voting_account_id.clone())
            .set_validator_stake(env::current_account_id(), self.total_staked_balance.into())
    }

    fn assert_owner(&self) {
        require!(env::predecessor_account_id() == self.owner_id, "Not owner");
    }
}
