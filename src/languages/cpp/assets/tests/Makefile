HOST_MAKEFILE_DIR := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
HOST_TEST_DIR := $(HOST_MAKEFILE_DIR)
HOST_ASSETS_DIR := $(abspath $(HOST_TEST_DIR)/..)

SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c

DOCKER_IMAGE ?= ptip-ffi-cpp-tests

TEST_FILES ?=
TESTCASES ?=

# Usage:
# - `make test`: Run all C++ unit tests found under the `testcases/` directory.
# - `make test TEST_FILES="testcases/foo.cpp bar.cpp": Only run the listed files.
# - `make test TESTCASES="bridge_test socket_test": Run only selected named test binaries.

test:
	@TEST_FILES="$(TEST_FILES)"; TESTCASES="$(TESTCASES)";
	docker build -t "$(DOCKER_IMAGE)" -f "$(HOST_TEST_DIR)/runner/Dockerfile" "$(HOST_TEST_DIR)" >/dev/null;
	docker run --rm -t \
		-e TEST_FILES="$$TEST_FILES" \
		-e TESTCASES="$$TESTCASES" \
		--workdir "/assets/tests" \
		-v "$(HOST_ASSETS_DIR):/assets" \
		"$(DOCKER_IMAGE)"
