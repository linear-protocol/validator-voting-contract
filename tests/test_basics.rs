use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn test_contract_is_operational() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;

    test_initialization(&contract_wasm).await?;
    Ok(())
}

async fn test_initialization(contract_wasm: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(contract_wasm).await?;

    // Initialize contract
    let proposal = "test_proposal";
    let deadline_timestamp_ms = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        + 10 * 60 * 1000) as u64;
    let _ = contract
        .call("new")
        .args_json(json!({
            "proposal": proposal,
            "deadline_timestamp_ms": deadline_timestamp_ms,
        }))
        .transact()
        .await?;

    let contract_deadline = contract
        .view("get_deadline_timestamp")
        .args_json(json!({}))
        .await?;
    assert_eq!(contract_deadline.json::<u64>()?, deadline_timestamp_ms);

    let contract_proposal = contract.view("get_proposal").args_json(json!({})).await?;
    assert_eq!(contract_proposal.json::<String>()?, proposal);

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
