#!/bin/bash
export VOTING_ACCOUNT_ID=reduce-inflation.testnet
export PROPOSAL_DESCRIPTION="reduce inflation rate"
export DEADLINE_TIMESTAMP=1749715200000

# create account
near account create-account sponsor-by-faucet-service $VOTING_ACCOUNT_ID autogenerate-new-keypair save-to-legacy-keychain network-config testnet create

# deploy contract
near contract deploy $VOTING_ACCOUNT_ID use-file validator_voting.wasm with-init-call new json-args '{"proposal":"'$PROPOSAL_DESCRIPTION'","deadline_timestamp_ms":'$DEADLINE_TIMESTAMP'}' prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-legacy-keychain send
