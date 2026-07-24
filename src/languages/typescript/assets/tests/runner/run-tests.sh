#!/bin/sh
set -eu

test_files=${TEST_FILES:-./testcases/*_test.ts}

if [ -z "${TESTCASES:-}" ]; then
  exec node --import=tsx --test ${test_files}
fi

test_names=$(printf '%s' "$TESTCASES" | tr ',' '|')
exec node --import=tsx --test --test-name-pattern "^(${test_names})$" ${test_files}