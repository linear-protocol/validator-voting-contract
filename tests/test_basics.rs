use serde_json::json;
use std::time::SystemTime;

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
    let deadline_timestamp_ms = SystemTime::now() + 10 * 60 * 1000;
    contract
        .call(contract.id(), "new")
        .args_json(json!({
            "proposal": proposal,
            "deadline_timestamp_md": deadline_timestamp_ms,
        }))
        .transact()
        .await?;

    let user_account = sandbox.dev_create_account().await?;
    let outcome = user_account
        .call(contract.id(), "vote")
        .args_json(json!({"is_vote": true}))
        .transact()
        .await?;
    assert!(outcome.is_failure(), "{:#?}", outcome.into_result().unwrap_err());

    let contract_proposal = contract.view("get_proposal").args_json(json!({})).await?;
    assert_eq!(contract_proposal.json::<String>()?, proposal);

    Ok(())
}
