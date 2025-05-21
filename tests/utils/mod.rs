use near_sdk::{near, AccountId, NearToken, PublicKey};
use near_workspaces::{network::Sandbox, Account, Contract, Worker};
use serde_json::json;
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[near(serializers = [json])]
pub struct VotingInitArgs {
    pub proposal: String,
    pub deadline_timestamp_ms: u64,
}

#[near(serializers = [json])]
pub struct MockStakingPoolInitArgs {
    pub owner_id: AccountId,
    pub stake_public_key: PublicKey,
    pub voting_account_id: AccountId,
}

pub async fn deploy_voting_contract(
    sandbox: &Worker<Sandbox>,
) -> Result<(Contract, VotingInitArgs), Box<dyn std::error::Error>> {
    let contract_wasm = std::fs::read("tests/res/validator_voting.wasm")?;
    let contract_account = create_account(sandbox, "voting", 100).await?;
    let contract = contract_account.deploy(&contract_wasm).await?.result;

    // Initialize contract
    let init_args = VotingInitArgs {
        proposal: "test_proposal".to_string(),
        deadline_timestamp_ms: (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 60 * 60 * 1000) as u64,
    };

    let _ = contract
        .call("new")
        .args_json(json!(init_args))
        .transact()
        .await?;

    Ok((contract, init_args))
}

pub async fn deploy_mock_staking_pool_contract(
    sandbox: &Worker<Sandbox>,
    voting_account_id: AccountId,
) -> Result<(Contract, Account, MockStakingPoolInitArgs), Box<dyn std::error::Error>> {
    let contract_wasm =
        near_workspaces::compile_project("./tests/contracts/mock-staking-pool").await?;
    let contract_account = create_account(sandbox, "staking-pool", 100).await?;
    let contract = contract_account.deploy(&contract_wasm).await?.result;

    let owner = create_account(sandbox, "owner", 10000).await?;
    let init_args = MockStakingPoolInitArgs {
        owner_id: owner.id().clone(),
        stake_public_key: PublicKey::from_str(
            "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
        )
        .unwrap(),
        voting_account_id,
    };
    let _ = contract
        .call("new")
        .args_json(json!(init_args))
        .transact()
        .await?;

    Ok((contract, owner, init_args))
}

pub async fn deploy_mock_staking_pool_contracts(
    sandbox: &Worker<Sandbox>,
    voting_account_id: AccountId,
    num: usize,
) -> Result<(Vec<Contract>, Account, MockStakingPoolInitArgs), Box<dyn std::error::Error>> {
    let contract_wasm =
        near_workspaces::compile_project("./tests/contracts/mock-staking-pool").await?;

    let owner = create_account(sandbox, "owner", 10000).await?;
    let init_args = MockStakingPoolInitArgs {
        owner_id: owner.id().clone(),
        stake_public_key: PublicKey::from_str(
            "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp",
        )
        .unwrap(),
        voting_account_id,
    };

    let mut contracts = Vec::<Contract>::with_capacity(num);
    for i in 0..num {
        let contract_account = create_account(sandbox, &format!("staking-pool-{i}"), 100).await?;
        let contract = contract_account.deploy(&contract_wasm).await?.result;
        let outcome = contract
            .call("new")
            .args_json(json!(init_args))
            .transact()
            .await?;
        assert!(
            outcome.is_success(),
            "{:#?}",
            outcome.into_result().unwrap_err()
        );
        contracts.push(contract);
    }

    Ok((contracts, owner, init_args))
}

pub async fn setup_env(
) -> Result<(Contract, Contract, Worker<Sandbox>, Account), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let (voting_contract, _) = deploy_voting_contract(&sandbox).await?;
    let (staking_pool_contract, owner, _) =
        deploy_mock_staking_pool_contract(&sandbox, voting_contract.id().clone()).await?;

    Ok((staking_pool_contract, voting_contract, sandbox, owner))
}

pub async fn setup_env_many(
    staking_pool_num: usize,
) -> Result<(Vec<Contract>, Contract, Worker<Sandbox>, Account), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let (voting_contract, _) = deploy_voting_contract(&sandbox).await?;
    let (staking_pool_contracts, owner, _) = deploy_mock_staking_pool_contracts(
        &sandbox,
        voting_contract.id().clone(),
        staking_pool_num,
    )
    .await?;

    Ok((staking_pool_contracts, voting_contract, sandbox, owner))
}

pub async fn create_account(
    sandbox: &Worker<Sandbox>,
    prefix: &str,
    balance: u128,
) -> Result<Account, Box<dyn std::error::Error>> {
    let root = sandbox.root_account().unwrap();
    Ok(root
        .create_subaccount(prefix)
        .initial_balance(NearToken::from_near(balance))
        .transact()
        .await?
        .result)
}
