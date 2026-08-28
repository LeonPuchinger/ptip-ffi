#!/bin/bash
set -euo pipefail

cd /assets/tests

if [ -n "${TEST_FILES:-}" ]; then
  echo "Running selected tests: ${TEST_FILES}"
  for file in ${TEST_FILES}; do
    g++ -std=c++20 -Wall -Wextra -pedantic "$file" -o /tmp/test_bin
    /tmp/test_bin
  done
else
  find . -maxdepth 2 -type f -name '*.cpp' -print0 | while IFS= read -r -d '' file; do
    echo "Compiling $file"
    g++ -std=c++20 -Wall -Wextra -pedantic "$file" -o /tmp/test_bin
    /tmp/test_bin
  done
fi
