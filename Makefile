check: test clippy format_check
.PHONY: check

test:
	cargo test --all-targets
.PHONY: test

coverage:
	cargo +nightly llvm-cov test --all-targets --branch --quiet --lcov --output-path=target/lcov.info
	cargo +nightly llvm-cov report --html --output-dir=target/coverage
	cargo +nightly llvm-cov report --summary-only
.PHONY: coverage

clippy:
	cargo clippy --all-targets
.PHONY: clippy

format_check:
	cargo fmt -- --check
.PHONY: format_check

Cargo.lock: Cargo.toml
	cargo check

LICENSE-3rdparty.yml: Cargo.toml Cargo.lock
	cargo bundle-licenses --format=yaml --output=LICENSE-3rdparty.yml
