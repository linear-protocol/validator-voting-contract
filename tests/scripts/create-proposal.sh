#!/bin/bash
export SUFFIX=delta
export MOCK_PROPOSAL_CONTRACT=./res/validator_voting.wasm
export VOTING_ACCOUNT_ID="mock-proposal-"$SUFFIX".testnet"
export PROPOSAL_DESCRIPTION="reduce inflation rate"
export DEADLINE_TIMESTAMP=1753430400000
export INIT_ARGS='{"proposal":"'$PROPOSAL_DESCRIPTION'","deadline_timestamp_ms":'$DEADLINE_TIMESTAMP'}'

echo $ARGS

# create account
near account create-account sponsor-by-faucet-service $VOTING_ACCOUNT_ID autogenerate-new-keypair save-to-legacy-keychain network-config testnet create

# deploy contract
near contract deploy $VOTING_ACCOUNT_ID use-file $MOCK_PROPOSAL_CONTRACT with-init-call new json-args "$INIT_ARGS" prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-legacy-keychain send
