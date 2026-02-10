SHELL := /usr/bin/env bash

.PHONY: ci-lint ci-build

# Run strict linting (requires system OIIO/OCIO dev packages).
ci-lint:
	cargo clippy --workspace --all-targets --features crispen-demo/ocio -- -D warnings

# Build and test the full workspace including OCIO features.
ci-build:
	cargo build --workspace --features crispen-demo/ocio
	cargo test --workspace --features crispen-demo/ocio
