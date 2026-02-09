SHELL := /usr/bin/env bash

.PHONY: ci-lint ci-build-ocio

# Run strict linting in check-only mode without requiring native OCIO deps.
ci-lint:
	CRISPEN_OCIO_SKIP_NATIVE_BUILD=1 cargo clippy --workspace --all-targets -- -D warnings

# Build the OCIO-enabled demo using a prebuilt OCIO install prefix.
# Usage:
#   make ci-build-ocio PREBUILT_OCIO_DIR=/path/to/opencolorio-install
ci-build-ocio:
	@if [[ -z "$(PREBUILT_OCIO_DIR)" ]]; then \
		echo "error: PREBUILT_OCIO_DIR is required"; \
		echo "example: make ci-build-ocio PREBUILT_OCIO_DIR=/opt/opencolorio"; \
		exit 1; \
	fi
	@if [[ ! -d "$(PREBUILT_OCIO_DIR)/include" ]]; then \
		echo "error: missing include/ under PREBUILT_OCIO_DIR: $(PREBUILT_OCIO_DIR)"; \
		exit 1; \
	fi
	@if [[ ! -d "$(PREBUILT_OCIO_DIR)/lib" && ! -d "$(PREBUILT_OCIO_DIR)/lib64" ]]; then \
		echo "error: missing lib/ or lib64/ under PREBUILT_OCIO_DIR: $(PREBUILT_OCIO_DIR)"; \
		exit 1; \
	fi
	CRISPEN_OCIO_PREBUILT_DIR="$(PREBUILT_OCIO_DIR)" cargo build -p crispen-demo --features ocio
