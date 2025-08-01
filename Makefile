check: test clippy format_check
.PHONY: check

test:
	cargo test --all-targets
.PHONY: test

coverage:
	cargo +nightly llvm-cov test --all-targets --branch --quiet --lcov --output-path=target/lcov.info \
		--fail-under-lines=85
	cargo +nightly llvm-cov report --html --output-dir=target/coverage
ifndef GITHUB_STEP_SUMMARY
	cargo +nightly llvm-cov report --summary-only
else
	echo "## Coverage Report"                     >> ${GITHUB_STEP_SUMMARY}
	echo ""                                       >> ${GITHUB_STEP_SUMMARY}
	echo '```'                                    >> ${GITHUB_STEP_SUMMARY}
	cargo +nightly llvm-cov report --summary-only >> ${GITHUB_STEP_SUMMARY}
	echo '```'                                    >> ${GITHUB_STEP_SUMMARY}
endif
.PHONY: coverage

clippy:
	cargo clippy --all-targets
.PHONY: clippy

format_check:
	cargo fmt -- --check
.PHONY: format_check

Cargo.lock: Cargo.toml
	cargo check

LICENSE-3rdparty.csv: Cargo.toml Cargo.lock
	cargo install --locked dd-rust-license-tool
	dd-rust-license-tool write
