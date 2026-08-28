SHELL := /bin/bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c

DOCKER_IMAGE ?= ptip-ffi-cpp-tests

TEST_FILES ?=
TESTCASES ?=

test:
	@TEST_FILES="$(TEST_FILES)"; TESTCASES="$(TESTCASES)";
	docker build -t "$(DOCKER_IMAGE)" -f "$(CURDIR)/runner/Dockerfile" "$(CURDIR)" >/dev/null;
	docker run --rm -t \
		-e TEST_FILES="$$TEST_FILES" \
		-e TESTCASES="$$TESTCASES" \
		--workdir "/assets/tests" \
		-v "$(CURDIR):/assets" \
		"$(DOCKER_IMAGE)"
