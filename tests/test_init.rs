use serde_json::json;

mod utils;
use utils::*;

// #[tokio::test]
async fn test_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let (contract, init_args) = deploy_voting_contract(&sandbox).await?;

    let contract_deadline = contract
        .view("get_deadline_timestamp")
        .args_json(json!({}))
        .await?;
    assert_eq!(
        contract_deadline.json::<u64>()?,
        init_args.deadline_timestamp_ms
    );

    let contract_proposal = contract.view("get_proposal").args_json(json!({})).await?;
    assert_eq!(contract_proposal.json::<String>()?, init_args.proposal);

    Ok(())
}
