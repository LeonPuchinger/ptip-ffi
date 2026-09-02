#!/usr/bin/env bash
set -Eeuo pipefail

readonly SMOKE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd -- "$SMOKE_ROOT/../.." && pwd)"
readonly OUTPUT_ROOT="$SMOKE_ROOT/output"

ALL_LANGUAGES=(py ts cpp)

usage() {
    cat <<'EOF'
Usage: test/smoke/test.sh [OPTIONS]

Run the smoke-test matrix. Without filters, all caller/callee permutations run.

Options:
  --caller-languages LANGUAGES  Filter callers (py, ts, cpp; comma-separated)
  --callee-languages LANGUAGES  Filter callees (py, ts, cpp; comma-separated)
  -h, --help                   Show this help
EOF
}

fail() {
    echo "error: $*" >&2
    exit 2
}

parse_languages() {
    local value="$1"
    local language
    local -a parsed=()
    IFS=',' read -r -a values <<< "$value"
    for language in "${values[@]}"; do
        language="${language,,}"
        case "$language" in
            py|python) parsed+=(py) ;;
            ts|typescript) parsed+=(ts) ;;
            cpp|c++) parsed+=(cpp) ;;
            *) fail "unknown language '$language' (expected py, ts, or cpp)" ;;
        esac
    done
    printf '%s\n' "${parsed[@]}"
}

contains_language() {
    local needle="$1"
    shift
    local language
    for language in "$@"; do
        [[ "$language" == "$needle" ]] && return 0
    done
    return 1
}

run_usage() {
    local output_dir="$1"
    shift
    local log_file="$output_dir/.smoke.log"
    local process_id
    local exit_code

    set +e
    setsid timeout --signal=TERM --kill-after=5s 60s "$@" >"$log_file" 2>&1 &
    process_id=$!
    wait "$process_id"
    exit_code=$?
    kill -TERM -- "-$process_id" 2>/dev/null || true
    wait "$process_id" 2>/dev/null || true
    set -e

    cat "$log_file"
    if [[ "$exit_code" -eq 0 ]]; then
        return 0
    fi
    if [[ "$exit_code" -eq 124 ]] && grep -q "integration passed" "$log_file"; then
        return 0
    fi
    return "$exit_code"
}

run_case() {
    local caller="$1"
    local callee="$2"
    local case_name="${caller}_${callee}"
    local output_dir="$OUTPUT_ROOT/$case_name"
    local caller_directory
    local callee_directory
    case "$caller" in
        py) caller_directory=python ;;
        ts) caller_directory=typescript ;;
        cpp) caller_directory=cpp ;;
    esac
    case "$callee" in
        py) callee_directory=python ;;
        ts) callee_directory=typescript ;;
        cpp) callee_directory=cpp ;;
    esac
    local input_dir="$SMOKE_ROOT/input/$callee_directory"
    local usage_source="$SMOKE_ROOT/usage/$caller_directory/usage.$([[ "$caller" == cpp ]] && echo cpp || echo "$caller")"
    local entry_point
    local library_language
    local output_language
    local command

    echo "==> $case_name"
    rm -rf -- "$output_dir"

    case "$callee" in
        py)
            entry_point="$input_dir/index.py"
            library_language="Python"
            ;;
        ts)
            entry_point="$input_dir/index.ts"
            library_language="TypeScript"
            ;;
        cpp)
            entry_point="$input_dir/index.hpp"
            library_language="C++"
            ;;
    esac

    cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- generate \
        --library-root "$input_dir" \
        --library-language "$library_language" \
        --library-entry-point "$entry_point" \
        --output-directory "$output_dir" \
        --output-language "$(case "$caller" in py) echo Python ;; ts) echo TypeScript ;; cpp) echo C++ ;; esac)"

    cp -- "$usage_source" "$output_dir/usage.$([[ "$caller" == cpp ]] && echo cpp || echo "$caller")"

    if [[ "$callee" == cpp ]]; then
        g++ -std=c++20 \
            -I"$output_dir/callee" -I"$output_dir" \
            -o "$output_dir/cpp_callee" "$output_dir/callee/main.cpp"
    fi

    if [[ "$caller" == cpp ]]; then
        g++ -std=c++20 \
            -I"$output_dir/caller" -I"$output_dir" \
            -o "$output_dir/cpp_usage" "$output_dir/usage.cpp"
    fi

    if [[ "$caller" == ts || "$callee" == ts ]]; then
        if [[ "$caller" == ts ]]; then
            npm install --prefix "$output_dir/caller" --ignore-scripts --silent
            npm rebuild --prefix "$output_dir/caller" synchronous-socket --silent
        fi
        if [[ "$callee" == ts ]]; then
            npm install --prefix "$output_dir/callee" --ignore-scripts --silent
            npm rebuild --prefix "$output_dir/callee" synchronous-socket --silent
        fi
    fi

    case "$callee" in
        py) command="python3 $output_dir/callee/main.py" ;;
        ts) command="tsx $output_dir/callee/main.ts" ;;
        cpp) command="$output_dir/cpp_callee" ;;
    esac

    case "$caller" in
        py)
            run_usage "$output_dir" env FFI_LIBRARY_INVOKE="$command" PYTHONPATH="$output_dir" python3 "$output_dir/usage.py"
            ;;
        ts)
            run_usage "$output_dir" env FFI_LIBRARY_INVOKE="$command" tsx "$output_dir/usage.ts"
            ;;
        cpp)
            run_usage "$output_dir" env FFI_LIBRARY_INVOKE="$command" "$output_dir/cpp_usage"
            ;;
    esac

    echo "<== $case_name passed"
}

caller_languages=()
callee_languages=()
while (($# > 0)); do
    case "$1" in
        --caller-languages|--callee-languages)
            (($# >= 2)) || fail "$1 requires a value"
            parsed_languages=( $(parse_languages "$2") )
            if [[ "$1" == --caller-languages ]]; then
                caller_languages+=("${parsed_languages[@]}")
            else
                callee_languages+=("${parsed_languages[@]}")
            fi
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            fail "unknown option '$1'"
            ;;
    esac
done

((${#caller_languages[@]})) || caller_languages=("${ALL_LANGUAGES[@]}")
((${#callee_languages[@]})) || callee_languages=("${ALL_LANGUAGES[@]}")

for caller in "${caller_languages[@]}"; do
    for callee in "${callee_languages[@]}"; do
        run_case "$caller" "$callee"
    done
done

echo "Smoke matrix passed: ${#caller_languages[@]} caller(s) x ${#callee_languages[@]} callee(s)"
