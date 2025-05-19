use crate::Balance;

#[cfg(feature = "integration-test")]
pub fn get_validators() -> std::collections::HashMap<near_sdk::AccountId, Balance> {
    near_sdk::env::storage_read(b"__validators__")
        .map_or_else(std::collections::HashMap::new, |validators| {
            near_sdk::borsh::from_slice(&validators).unwrap()
        })
}

#[cfg(feature = "integration-test")]
pub fn set_validators(validators: std::collections::HashMap<near_sdk::AccountId, Balance>) {
    near_sdk::env::storage_write(
        b"__validators__",
        &near_sdk::borsh::to_vec(&validators).unwrap(),
    );
}

#[cfg(feature = "integration-test")]
pub fn get_validator_stake(validator_account_id: &near_sdk::AccountId) -> Balance {
    let validators = get_validators();
    validators
        .get(validator_account_id)
        .copied()
        .unwrap_or_default()
}

#[cfg(feature = "integration-test")]
pub fn set_validator_stake(validator_account_id: near_sdk::AccountId, amount: Balance) {
    let mut validators = get_validators();
    validators.insert(validator_account_id, amount);
    set_validators(validators);
}

pub fn validator_stake(validator_account_id: &near_sdk::AccountId) -> Balance {
    #[cfg(feature = "integration-test")]
    return get_validator_stake(validator_account_id);
    #[cfg(not(feature = "integration-test"))]
    near_sdk::env::validator_stake(validator_account_id).as_yoctonear()
}

pub fn validator_total_stake() -> Balance {
    #[cfg(feature = "integration-test")]
    return get_validators().values().sum();
    #[cfg(not(feature = "integration-test"))]
    near_sdk::env::validator_total_stake().as_yoctonear()
}
