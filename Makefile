check: test miri clippy format_check
.PHONY: check

test:
	cargo test --features serde_test
.PHONY: test

miri:
	cargo +nightly miri test --features serde_test
.PHONY: miri

coverage:
	cargo +nightly llvm-cov test --all-targets --branch --features=serde_test --quiet --lcov --output-path=target/lcov.info
	cargo +nightly llvm-cov report --html --output-dir=target/coverage
	cargo +nightly llvm-cov report --summary-only
.PHONY: coverage

clippy:
	cargo clippy --all-targets --features=serde_test
.PHONY: clippy

format_check:
	cargo fmt -- --check
.PHONY: format_check
