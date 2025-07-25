#!/bin/bash
export SUFFIX=delta
export OWNER_ID=mock-owner.testnet
export VOTING_ACCOUNT_ID="mock-proposal-"$SUFFIX".testnet"

for i in {1..1}; do
    VALIDATOR_ID="mock-node-"$SUFFIX"-"${i}".testnet"
    # vote by validator
    near contract call-function as-transaction $VOTING_ACCOUNT_ID vote json-args '{"staking_pool_id":"'$VALIDATOR_ID'","vote":"yes"}' prepaid-gas '200.0 Tgas' attached-deposit '0 NEAR' sign-as $OWNER_ID network-config testnet sign-with-legacy-keychain send
done

# get total voted stake
near contract call-function as-read-only $VOTING_ACCOUNT_ID get_total_voted_stake json-args {} network-config testnet now
# get votes
near contract call-function as-read-only $VOTING_ACCOUNT_ID get_votes json-args {} network-config testnet now
