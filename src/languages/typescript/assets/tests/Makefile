# These paths refer to the host filesystem, not the container's filesystem.
HOST_MAKEFILE_DIR := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
HOST_TEST_DIR := $(HOST_MAKEFILE_DIR)
HOST_ASSETS_DIR := $(abspath $(HOST_TEST_DIR)/..)

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c

DOCKER_IMAGE ?= ptip-ffi-typescript-tests

# Usage:
# - `make test`: Run all tests. Test discovery is delegated to Node's test runner.
# - `make test TEST_FILES="foo.ts bar.ts": Only execute tests in the specified
#		files (or directories). Use paths relative to the `tests/` directory only.
# - `make test TESTCASES="foo,bar,foo bar"`: Only execute test cases with names exactly
#		matching the specified, comma-separated names.
test:
	@# Convert make variables to shell variables
	TEST_FILES="$(TEST_FILES)"
	TESTCASES="$(TESTCASES)"
	export TEST_FILES TESTCASES
	docker build -t "$(DOCKER_IMAGE)" -f "$(HOST_TEST_DIR)/Dockerfile" "$(HOST_ASSETS_DIR)" >/dev/null; \
	docker run --rm -t \
		-e TEST_FILES="$$TEST_FILES" \
		-e TESTCASES="$$TESTCASES" \
		--workdir "/assets/tests" \
		-v "$(HOST_ASSETS_DIR):/assets" \
		"$(DOCKER_IMAGE)"
