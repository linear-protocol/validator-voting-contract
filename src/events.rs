use near_sdk::json_types::{U128, U64};
use near_sdk::serde::Serialize;
use near_sdk::serde_json::json;
use near_sdk::{log, AccountId};

pub const EVENT_STANDARD: &str = "validator-voting";
pub const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize)]
#[serde(
    crate = "near_sdk::serde",
    rename_all = "snake_case",
    tag = "event",
    content = "data"
)]
#[must_use = "Don't forget to `.emit()` this event"]
pub enum Event<'a> {
    Voted {
        validator_id: &'a AccountId,
        choice: &'a Choice,
    },
    ProposalApproved {
        proposal: &'a String,
        approval_timestamp_ms: &'a U64,
        deadline_timestamp_ms: &'a U64,
        voted_stake: &'a U128,
        total_stake: &'a U128,
        num_votes: &'a U64,
    },
}

impl Event<'_> {
    pub fn emit(&self) {
        let json = json!(self);
        let event_json = json!({
            "standard": EVENT_STANDARD,
            "version": EVENT_STANDARD_VERSION,
            "event": json["event"],
            "data": [json["data"]]
        })
        .to_string();
        log!("EVENT_JSON:{}", event_json);
    }
}
