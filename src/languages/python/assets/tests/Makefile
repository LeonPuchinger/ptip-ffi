HOST_MAKEFILE_DIR := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
HOST_TEST_DIR := $(HOST_MAKEFILE_DIR)
HOST_ASSETS_DIR := $(abspath $(HOST_TEST_DIR)/..)

SHELL := /bin/sh
.ONESHELL:
.SHELLFLAGS := -eu -c

DOCKER_IMAGE ?= ptip-ffi-python-tests

# Usage:
# - `make test`: Run all Python asset tests through Docker.
# - `make test TESTCASES="bridge_test socket_test"`: Run a subset of test module names.

test:
	TESTCASES="$(TESTCASES)"
	docker build -t "$(DOCKER_IMAGE)" -f "$(HOST_TEST_DIR)/runner/Dockerfile" "$(HOST_TEST_DIR)" >/dev/null
	docker run --rm -t \
		-e TESTCASES="$$TESTCASES" \
		--workdir "/assets/tests" \
		-v "$(HOST_ASSETS_DIR):/assets" \
		"$(DOCKER_IMAGE)"
