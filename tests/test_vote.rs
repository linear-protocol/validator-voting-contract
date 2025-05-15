use near_sdk::{Gas, NearToken};
use serde_json::json;

mod utils;
use utils::*;

#[tokio::test]
async fn test_non_validator_cannot_vote() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let (contract, _) = deploy_voting_contract(&sandbox).await?;

    let sandbox = near_workspaces::sandbox().await?;
    let user_account = sandbox.dev_create_account().await?;
    let outcome = user_account
        .call(contract.id(), "vote")
        .args_json(json!({"is_vote": true}))
        .transact()
        .await?;
    assert!(
        outcome.is_failure(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    Ok(())
}

#[tokio::test]
async fn test_simple_vote() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contract, voting_contract, sandbox, owner) = setup_env().await?;

    let alice = create_account(&sandbox, "alice", 10000).await?;
    let outcome = alice
        .call(staking_pool_contract.id(), "stake")
        .args_json(json!({}))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let staked_balance = staking_pool_contract
        .view("get_staked_balance")
        .args_json(json!({}))
        .await?;
    println!(
        "user account: {}, {:#?}",
        alice.id(),
        staked_balance.json::<(String, String)>()?
    );

    let outcome = owner
        .call(staking_pool_contract.id(), "vote")
        .args_json(json!({
            "voting_account_id": voting_contract.id(),
            "is_vote": true
        }))
        .gas(Gas::from_tgas(200))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    Ok(())
}
