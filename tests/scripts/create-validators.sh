#!/bin/bash
export OWNER_ID=mock-owner.testnet
export STAKE_ACCOUNT_ID=mock-staker.testnet
export VOTING_ACCOUNT_ID=reduce-inflation.testnet
export STAKE_PUBLIC_KEY=ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp

for i in {1..1}; do
    VALIDATOR_ID="mock-node-"${i}".testnet"
    # create validator account
    # near account create-account sponsor-by-faucet-service $VALIDATOR_ID autogenerate-new-keypair save-to-legacy-keychain network-config testnet create
    near account create-account fund-myself $VALIDATOR_ID '2 NEAR' autogenerate-new-keypair save-to-legacy-keychain sign-as $STAKE_ACCOUNT_ID network-config testnet sign-with-legacy-keychain send
    # deploy mock staking pool contract
    near contract deploy $VALIDATOR_ID use-file mock_staking_pool.wasm with-init-call new json-args '{"owner_id":"'$OWNER_ID'","stake_public_key":"'$STAKE_PUBLIC_KEY'","voting_account_id":"'$VOTING_ACCOUNT_ID'"}' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-legacy-keychain send
    # stake some NEAR to 
    near contract call-function as-transaction $VALIDATOR_ID deposit_and_stake json-args {} prepaid-gas '200.0 Tgas' attached-deposit '1 NEAR' sign-as $STAKE_ACCOUNT_ID network-config testnet sign-with-legacy-keychain send
done
