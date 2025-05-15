use near_sdk::near;
use near_workspaces::{network::Sandbox, Contract, Worker};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[near(serializers = [json])]
pub struct InitArgs {
    pub proposal: String,
    pub deadline_timestamp_ms: u64,
}

pub async fn deploy_voting_contract(
) -> Result<(Contract, Worker<Sandbox>, InitArgs), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // Initialize contract
    let init_args = InitArgs {
        proposal: "test_proposal".to_string(),
        deadline_timestamp_ms: (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 10 * 60 * 1000) as u64,
    };

    let _ = contract
        .call("new")
        .args_json(json!(init_args))
        .transact()
        .await?;

    Ok((contract, sandbox, init_args))
}
