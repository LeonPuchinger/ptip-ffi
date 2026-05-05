# These paths refer to the host filesystem, not the container's filesystem.
HOST_MAKEFILE_DIR := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
HOST_TEST_DIR := $(HOST_MAKEFILE_DIR)
HOST_ASSETS_DIR := $(abspath $(HOST_TEST_DIR)/..)
# These paths refer to the container's filesystem.
CONTAINER_TEST_DIR := /assets/tests

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c

DOCKER_IMAGE ?= denoland/deno:latest

# Usage:
# - `make test`: Run all tests. Test discovery is delegated to the testing framework.
# - `make test TEST_FILES="foo.ts bar.ts`: Only execute tests in the specified
#		files (or directories). Use paths relative to the `tests/` directory only.
# - `make test TESTCASES="foo,bar,foo bar"`: Only execute test cases with names exactly
#		matching the specified, comma-separated names.
test:
	@# Convert make variables to shell variables
	TEST_FILES="$(TEST_FILES)"
	TESTCASES="$(TESTCASES)"
	if [ -z "$${TEST_FILES}" ]; then \
		TEST_FILES="$(CONTAINER_TEST_DIR)"; \
	fi; \
	if [ -z "$${TESTCASES}" ]; then \
		FILTER_OPERATOR=""; \
		FILTER_PATTERN=""; \
	else \
		FILTER_OPERATOR="--filter"; \
		FILTER_PATTERN="/^($${TESTCASES//,/|})$$/"; \
	fi;
	docker run --rm -t \
		-v "$(HOST_ASSETS_DIR):/assets" \
		--workdir "/assets/tests" \
		"$(DOCKER_IMAGE)" \
		test --allow-all $$TEST_FILES $${FILTER_OPERATOR} "$${FILTER_PATTERN}"
