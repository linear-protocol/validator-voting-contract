use near_sdk::{AccountId, Gas, NearToken};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

mod utils;
use utils::*;

#[tokio::test]
async fn test_non_validator_cannot_vote() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let (contract, _) = deploy_voting_contract(
        &sandbox,
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 10 * 60 * 1000) as u64,
    )
    .await?;

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
    let (staking_pool_contract, voting_contract, sandbox, owner) = setup_env(None).await?;

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
#[ignore = "test is time consuming"] // Test takes around 50 minutes
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
        sandbox.fast_forward(500).await?;
        let block = sandbox.view_block().await?;

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
        println!("validator #{} voted at epoch ({})", index, block.epoch_id());
        println!(
            "Votes: {:#?}",
            voting_contract
                .view("get_votes")
                .await?
                .json::<HashMap<AccountId, String>>()?
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_vote_expiration() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contract, voting_contract, sandbox, owner) = setup_env(Some(
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 60 * 1000) as u64,
    ))
    .await?;

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

    sandbox.fast_forward(500).await?; // let vote expire

    let outcome = owner
        .call(staking_pool_contract.id(), "vote")
        .args_json(json!({
            "voting_account_id": voting_contract.id(),
            "is_vote": true
        }))
        .gas(Gas::from_tgas(200))
        .transact()
        .await?;

    assert!(outcome
        .into_result()
        .unwrap_err()
        .to_string()
        .contains("Smart contract panicked: Voting deadline has already passed"));

    Ok(())
}

#[tokio::test]
async fn test_withdraw_vote() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contracts, voting_contract, sandbox, owner) = setup_env_many(2).await?;

    let alice = create_account(&sandbox, "alice", 10000).await?;

    let outcome = alice
        .call(staking_pool_contracts[0].id(), "deposit_and_stake")
        .gas(Gas::from_tgas(250))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let outcome = alice
        .call(staking_pool_contracts[1].id(), "deposit_and_stake")
        .gas(Gas::from_tgas(250))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let outcome = owner
        .call(staking_pool_contracts[0].id(), "vote")
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

    let votes = owner.view(voting_contract.id(), "get_votes").await?;
    assert_eq!(votes.json::<HashMap<AccountId, String>>()?.len(), 1);

    let outcome = owner
        .call(staking_pool_contracts[0].id(), "vote")
        .args_json(json!({
            "voting_account_id": voting_contract.id(),
            "is_vote": false
        }))
        .gas(Gas::from_tgas(200))
        .transact()
        .await?;

    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let votes = owner.view(voting_contract.id(), "get_votes").await?;
    assert_eq!(votes.json::<HashMap<AccountId, String>>()?.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_unstake_after_voting() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contracts, voting_contract, sandbox, owner) = setup_env_many(2).await?;

    let alice = create_account(&sandbox, "alice", 10000).await?;

    let outcome = alice
        .call(staking_pool_contracts[0].id(), "deposit_and_stake")
        .gas(Gas::from_tgas(250))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let outcome = alice
        .call(staking_pool_contracts[1].id(), "deposit_and_stake")
        .gas(Gas::from_tgas(250))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let outcome = owner
        .call(staking_pool_contracts[0].id(), "vote")
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

    let outcome = alice
        .call(staking_pool_contracts[0].id(), "unstake")
        .args_json(json!({
            "amount": NearToken::from_near(1000).as_yoctonear().to_string()
        }))
        .gas(Gas::from_tgas(250))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );

    let votes = owner.view(voting_contract.id(), "get_votes").await?;
    let votes = votes.json::<HashMap<AccountId, String>>()?;
    assert_eq!(votes.len(), 1);
    assert!(votes.contains_key(staking_pool_contracts[0].id()));

    sandbox.fast_forward(500).await?;

    let outcome = owner
        .call(staking_pool_contracts[1].id(), "vote")
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

    let votes = owner.view(voting_contract.id(), "get_votes").await?;
    let votes = votes.json::<HashMap<AccountId, String>>()?;
    assert_eq!(votes.len(), 1);
    assert!(votes.contains_key(staking_pool_contracts[1].id()));

    Ok(())
}
