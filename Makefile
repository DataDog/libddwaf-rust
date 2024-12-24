test:
	cargo test --features serde_test
.PHONY: miri

miri:
	cargo +nightly miri test --features serde_test
.PHONY: miri

coverage:
	cargo tarpaulin --out Html --features serde_test
.PHONY: coverage

clippy:
	cargo clippy
.PHONY: clippy
