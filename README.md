# NEAR Validator Voting

The purpose of this contract is for validators to vote on any specific proposal. Validators can call `vote` function to vote for yes or no with the staked amount on the validator. If there are more than 2/3 of the stake at any given moment voting for yes, the voting is done. After the voting is finished or the voting deadline has passed, no one can further modify the contract. The voting contract is recommended to be pinged every epoch to make sure the latest stake is updated in the contract.

## Build

Install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near build
```

## Test

```bash
cargo test
```

## Deploy

```bash
cargo near deploy build-reproducible-wasm <account-id>
```

## Tools

- [cargo-near](https://github.com/near/cargo-near) - NEAR smart contract development toolkit for Rust
- [near CLI](https://near.cli.rs) - Interact with NEAR blockchain from command line
- [NEAR Rust SDK Documentation](https://docs.near.org/sdk/rust/introduction)
