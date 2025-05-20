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
    assert!(outcome.is_failure(),);

    Ok(())
}

#[tokio::test]
async fn test_simple_vote() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contract, voting_contract, sandbox, owner) = setup_env().await?;

    let alice = create_account(&sandbox, "alice", 10000).await?;
    let outcome = alice
        .call(staking_pool_contract.id(), "deposit_and_stake")
        .args_json(json!({}))
        .gas(Gas::from_tgas(250))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let staked_balance = voting_contract
        .view("get_validator_stake")
        .args_json(json!({
            "validator_account_id": staking_pool_contract.id()
        }))
        .await?;
    println!(
        "user account: {}, {:#?}",
        alice.id(),
        staked_balance.json::<String>()?
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

#[tokio::test]
async fn test_many_votes() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contracts, voting_contract, sandbox, owner) = setup_env_many(300).await?;

    let alice = create_account(&sandbox, "alice", 50000).await?;

    for staking_pool_contract in staking_pool_contracts.iter() {
        let outcome = alice
            .call(staking_pool_contract.id(), "deposit_and_stake")
            .args_json(json!({}))
            .gas(Gas::from_tgas(250))
            .deposit(NearToken::from_near(100))
            .transact()
            .await?;
        assert!(
            outcome.is_success(),
            "{:#?}",
            outcome.into_result().unwrap_err()
        );

        let total_staked = voting_contract.view("get_validator_total_stake").await?;
        println!(
            "total staked: {}, {:#?}",
            alice.id(),
            total_staked.json::<String>()?
        );
    }

    for (index, staking_pool_contract) in staking_pool_contracts.iter().enumerate() {
        let outcome = owner
            .call(voting_contract.id(), "ping")
            .gas(Gas::from_tgas(300))
            .transact()
            .await?;

        if index <= 200 {
            assert!(
                outcome.is_success(),
                "{:#?}",
                outcome.into_result().unwrap_err()
            );
        } else {
            assert!(
                outcome.is_failure(),
                "Ping should failed since vote is finished",
            );
            break;
        }

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
        println!("validator #{index} voted");
    }

    Ok(())
}
