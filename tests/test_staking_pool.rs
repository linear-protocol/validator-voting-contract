use std::time::{SystemTime, UNIX_EPOCH};
use near_sdk::{Gas, NearToken};
use near_workspaces::AccountId;
use serde_json::json;
use crate::utils::{create_account, deploy_voting_contract, setup_env};

mod utils;

#[tokio::test]
async fn test_stake_unstake() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contract, _, sandbox, _) = setup_env(None).await?;
    let alice = create_account(&sandbox, "alice", 10000).await?;
    let outcome = alice
        .call(staking_pool_contract.id(), "deposit_and_stake")
        .gas(Gas::from_tgas(250))
        .deposit(NearToken::from_near(1000))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );
    let balance = alice.view_account().await?.balance.as_near();
    assert!(balance >= 8999 && balance < 9000);

    let outcome = alice
        .call(staking_pool_contract.id(), "unstake")
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
    let balance = alice.view_account().await?.balance.as_near();
    assert!(balance >= 9999 && balance < 10000);

    Ok(())
}

#[tokio::test]
async fn test_get_set_voting_account_id() -> Result<(), Box<dyn std::error::Error>> {
    let (staking_pool_contract, _, sandbox, owner) = setup_env(None).await?;
    let (new_voting_contract, _) = deploy_voting_contract(
        &sandbox,
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 10 * 60 * 1000) as u64,
    ).await?;

    let alice = create_account(&sandbox, "alice", 10000).await?;
    let outcome = alice
        .call(staking_pool_contract.id(), "deposit_and_stake")
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
        .call(staking_pool_contract.id(), "set_voting_account_id")
        .args_json(json!({
            "voting_account_id": new_voting_contract.id(),
        }))
        .transact()
        .await?;
    assert!(
        outcome.is_success(),
        "{:#?}",
        outcome.into_result().unwrap_err()
    );
    let voting_account_id = staking_pool_contract.view("get_voting_account_id").await?.json::<AccountId>()?;
    assert_eq!(&voting_account_id, new_voting_contract.id());

    let validator_stake = new_voting_contract.view("get_validator_stake")
        .args_json(json!({
            "validator_account_id": staking_pool_contract.id(),
        })).await?.json::<String>()?;

    assert_eq!(validator_stake, NearToken::from_near(1000).as_yoctonear().to_string());

    Ok(())
}
