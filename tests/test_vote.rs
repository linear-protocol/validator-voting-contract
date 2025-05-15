use serde_json::json;

mod utils;
use utils::*;

#[tokio::test]
async fn test_non_validator_cannot_vote() -> Result<(), Box<dyn std::error::Error>> {
    let (contract, sandbox, _) = deploy_voting_contract().await?;

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
