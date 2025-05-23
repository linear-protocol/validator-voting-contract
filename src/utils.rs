use crate::Balance;

#[cfg(feature = "integration-test")]
fn get_validators() -> near_sdk::store::LookupMap<near_sdk::AccountId, Balance> {
    near_sdk::env::storage_read("__validators_map__".as_bytes()).map_or_else(
        || near_sdk::store::LookupMap::new("__validators__".as_bytes()),
        |validators| near_sdk::borsh::from_slice(&validators).unwrap(),
    )
}

#[cfg(feature = "integration-test")]
fn set_validators(validators: near_sdk::store::LookupMap<near_sdk::AccountId, Balance>) {
    near_sdk::env::storage_write(
        "__validators_map__".as_bytes(),
        &near_sdk::borsh::to_vec(&validators).unwrap(),
    );
}

#[cfg(feature = "integration-test")]
fn get_validator_total_stake() -> Balance {
    near_sdk::env::storage_read("__validator_total_stake__".as_bytes())
        .map_or(0, |amount| near_sdk::borsh::from_slice(&amount).unwrap())
}

#[cfg(feature = "integration-test")]
fn set_validator_total_stake(amount: Balance) {
    near_sdk::env::storage_write(
        "__validator_total_stake__".as_bytes(),
        &near_sdk::borsh::to_vec(&amount).unwrap(),
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

    let old_amount = validators
        .get(&validator_account_id)
        .copied()
        .unwrap_or_default();

    let total = get_validator_total_stake();

    validators.insert(validator_account_id, amount);
    set_validators(validators);
    set_validator_total_stake(total + amount - old_amount);
}

pub fn validator_stake(validator_account_id: &near_sdk::AccountId) -> Balance {
    #[cfg(feature = "integration-test")]
    return get_validator_stake(validator_account_id);
    #[cfg(not(feature = "integration-test"))]
    near_sdk::env::validator_stake(validator_account_id).as_yoctonear()
}

pub fn validator_total_stake() -> Balance {
    #[cfg(feature = "integration-test")]
    return get_validator_total_stake();
    #[cfg(not(feature = "integration-test"))]
    near_sdk::env::validator_total_stake().as_yoctonear()
}
