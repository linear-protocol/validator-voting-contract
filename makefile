RUSTFLAGS = "-C link-arg=-s"

all: validator-voting validator-voting-integration-test

lint:
	@cargo fmt --all
	@cargo clippy --fix --allow-dirty --allow-staged

validator-voting:
	$(call compile-release,validator-voting)
	@mkdir -p res
	@cp target/near/validator_voting.wasm ./res/validator_voting.wasm

validator-voting-integration-test:
	$(call compile-release,validator-voting,integration-test)
	@mkdir -p tests/res
	@cp target/near/validator_voting.wasm ./tests/res/validator_voting.wasm

test:
	@cargo test

define compile-release
	@rustup target add wasm32-unknown-unknown
	@cargo near build non-reproducible-wasm $(if $(2),--features $(2))
endef
