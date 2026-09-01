#!/bin/bash
set -uo pipefail

cd /assets/tests

run_case() {
  local label="$1"
  local file="$2"
  local out_path="/tmp/${label}.out"
  local status=0

  echo "[CASE] ${label}"
  echo "  file: ${file}"

  if ! g++ -std=c++20 -Wall -Wextra -pedantic "$file" -I. -I./testcases -o "/tmp/${label}_bin" 2>"$out_path"; then
    echo "  RESULT: FAIL (compile error)"
    cat "$out_path"
    return 1
  fi

  if "/tmp/${label}_bin" >"$out_path" 2>&1; then
    echo "  RESULT: PASS"
    return 0
  else
    status=$?
    echo "  RESULT: FAIL (exit ${status})"
    cat "$out_path"
    return 1
  fi
}

selected=()

if [ -n "${TEST_CASES:-}" ] || [ -n "${TESTCASES:-}" ]; then
  names="${TEST_CASES:-${TESTCASES:-}}"
  for name in ${names//,/ }; do
    file="testcases/${name}.cpp"
    if [ ! -f "$file" ]; then
      echo "Missing test case: $file" >&2
      exit 1
    fi
    selected+=("${name}|${file}")
  done
elif [ -n "${TEST_FILES:-}" ]; then
  for file in ${TEST_FILES}; do
    if [ ! -f "$file" ]; then
      echo "Missing file: $file" >&2
      exit 1
    fi
    label="$(basename "$file" .cpp)"
    selected+=("${label}|${file}")
  done
else
  while IFS= read -r -d '' file; do
    label="$(basename "$file" .cpp)"
    selected+=("${label}|${file}")
  done < <(find ./testcases -maxdepth 1 -type f -name '*.cpp' -print0 | sort -z)
fi

if [ "${#selected[@]}" -eq 0 ]; then
  echo "No C++ test cases were selected."
  exit 1
fi

passed=0
failed=0

for entry in "${selected[@]}"; do
  IFS='|' read -r label file <<<"$entry"
  if run_case "$label" "$file"; then
    passed=$((passed + 1))
  else
    failed=$((failed + 1))
  fi
  echo
  echo "---"
  echo
  sleep 0.05
done

echo "SUMMARY: ${passed} passed, ${failed} failed, ${#selected[@]} total"
if [ "$failed" -ne 0 ]; then
  exit 1
fi
