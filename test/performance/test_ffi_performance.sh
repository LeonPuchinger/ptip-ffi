#!/usr/bin/env bash
set -Eeuo pipefail

readonly PERFORMANCE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd -- "$PERFORMANCE_ROOT/../.." && pwd)"
readonly OUTPUT_ROOT="$PERFORMANCE_ROOT/output"
readonly RESULTS_ROOT="$PERFORMANCE_ROOT/results"
readonly OPERATIONS="${FFI_PERFORMANCE_OPERATIONS:-100000}"

ALL_LANGUAGES=(py ts cpp)
repetitions=1

usage() {
    cat <<'EOF'
Usage: test/performance/test_ffi_performance [OPTIONS]

Run the FFI performance matrix. Without filters, all caller/callee permutations run.

Options:
  --caller-languages LANGUAGES  Filter callers (py, ts, cpp; comma-separated)
  --callee-languages LANGUAGES  Filter callees (py, ts, cpp; comma-separated)
  --repetitions COUNT           Run each selected case COUNT times (default: 1)
  -h, --help                    Show this help

Environment:
  FFI_PERFORMANCE_OPERATIONS    Operations per run (default: 100000)
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

run_once() {
    local output_dir="$1"
    local result_file="$2"
    shift 2
    local process_id
    local exit_code

    set +e
    setsid timeout --signal=TERM --kill-after=5s 300s "$@" &
    process_id=$!
    wait "$process_id"
    exit_code=$?
    kill -KILL -- "-$process_id" 2>/dev/null || true
    wait "$process_id" 2>/dev/null || true
    set -e
    return "$exit_code"
}

run_case() {
    local caller="$1"
    local callee="$2"
    local case_name="${caller}_${callee}"
    local output_dir="$OUTPUT_ROOT/$case_name"
    local result_file="$RESULTS_ROOT/ffi_${case_name}.csv"
    local caller_directory
    local callee_directory
    local usage_extension
    local input_dir
    local usage_source
    local entry_point
    local library_language
    local output_language
    local command

    case "$caller" in
        py) caller_directory=python; usage_extension=py; output_language=Python ;;
        ts) caller_directory=typescript; usage_extension=ts; output_language=TypeScript ;;
        cpp) caller_directory=cpp; usage_extension=cpp; output_language=C++ ;;
    esac
    case "$callee" in
        py) callee_directory=python; entry_point=index.py; library_language=Python ;;
        ts) callee_directory=typescript; entry_point=index.ts; library_language=TypeScript ;;
        cpp) callee_directory=cpp; entry_point=index.hpp; library_language=C++ ;;
    esac

    input_dir="$PERFORMANCE_ROOT/input/$callee_directory"
    usage_source="$PERFORMANCE_ROOT/usage/$caller_directory/usage.$usage_extension"

    echo "==> $case_name ($repetitions repetition(s), $OPERATIONS operations)"
    rm -rf -- "$output_dir"
    mkdir -p -- "$RESULTS_ROOT"

    cargo run --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml" -- generate \
        --library-root "$input_dir" \
        --library-language "$library_language" \
        --library-entry-point "$input_dir/$entry_point" \
        --output-directory "$output_dir" \
        --output-language "$output_language"

    cp -- "$usage_source" "$output_dir/usage.$usage_extension"

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
            npm rebuild --prefix "$output_dir/caller" synchronous-socket
            find "$output_dir/caller/node_modules/synchronous-socket" -name SynchronousSocket.node -print -quit | grep -q . \
                || fail "native synchronous-socket addon was not built for $case_name caller"
        fi
        if [[ "$callee" == ts ]]; then
            npm install --prefix "$output_dir/callee" --ignore-scripts --silent
            npm rebuild --prefix "$output_dir/callee" synchronous-socket
            find "$output_dir/callee/node_modules/synchronous-socket" -name SynchronousSocket.node -print -quit | grep -q . \
                || fail "native synchronous-socket addon was not built for $case_name callee"
        fi
    fi

    case "$callee" in
        py) command="python3 $output_dir/callee/main.py" ;;
        ts) command="tsx $output_dir/callee/main.ts" ;;
        cpp) command="$output_dir/cpp_callee" ;;
    esac

    for ((repetition = 1; repetition <= repetitions; repetition++)); do
        echo "  repetition $repetition/$repetitions"
        case "$caller" in
            py)
                run_once "$output_dir" "$result_file" env FFI_LIBRARY_INVOKE="$command" PYTHONPATH="$output_dir" \
                    python3 "$output_dir/usage.py" "$OPERATIONS" "$result_file"
                ;;
            ts)
                run_once "$output_dir" "$result_file" env FFI_LIBRARY_INVOKE="$command" \
                    tsx "$output_dir/usage.ts" "$OPERATIONS" "$result_file"
                ;;
            cpp)
                run_once "$output_dir" "$result_file" env FFI_LIBRARY_INVOKE="$command" \
                    "$output_dir/cpp_usage" "$OPERATIONS" "$result_file"
                ;;
        esac
    done

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
        --repetitions)
            (($# >= 2)) || fail "--repetitions requires a value"
            [[ "$2" =~ ^[1-9][0-9]*$ ]] || fail "--repetitions must be a positive integer"
            repetitions="$2"
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

[[ "$OPERATIONS" =~ ^[1-9][0-9]*$ ]] || fail "FFI_PERFORMANCE_OPERATIONS must be a positive integer"
((${#caller_languages[@]})) || caller_languages=("${ALL_LANGUAGES[@]}")
((${#callee_languages[@]})) || callee_languages=("${ALL_LANGUAGES[@]}")

for caller in "${caller_languages[@]}"; do
    for callee in "${callee_languages[@]}"; do
        run_case "$caller" "$callee"
    done
done

echo "Performance matrix passed: ${#caller_languages[@]} caller(s) x ${#callee_languages[@]} callee(s)"
