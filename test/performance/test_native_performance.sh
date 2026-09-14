#!/usr/bin/env bash
set -Eeuo pipefail

readonly PERFORMANCE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly OPERATIONS="${FFI_PERFORMANCE_OPERATIONS:-100000}"
readonly BUILD_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ptip-ffi-native-performance.XXXXXX")"

cleanup() {
    rm -rf -- "$BUILD_ROOT"
}
trap cleanup EXIT

ALL_LANGUAGES=(py ts cpp)
repetitions=1

usage() {
    cat <<'EOF'
Usage: test/performance/test_native_performance.sh [OPTIONS]

Run native performance tests. Without filters, all native language tests run.

Options:
  --languages LANGUAGES  Filter languages (py, ts, cpp; comma-separated)
  --repetitions COUNT    Run each selected test COUNT times (default: 1)
  -h, --help             Show this help

Environment:
  FFI_PERFORMANCE_OPERATIONS  Operations per run (default: 100000)
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

run_language() {
    local language="$1"
    local usage_dir="$PERFORMANCE_ROOT/native"
    local command

    case "$language" in
        py)
            command=(python3 "$usage_dir/python/usage.py" "$OPERATIONS")
            ;;
        ts)
            command=(tsx "$usage_dir/typescript/usage.ts" "$OPERATIONS")
            ;;
        cpp)
            local binary="$BUILD_ROOT/native_performance"
            g++ -std=c++20 -I"$PERFORMANCE_ROOT/input/cpp" \
                -o "$binary" "$usage_dir/cpp/usage.cpp"
            command=("$binary" "$OPERATIONS")
            ;;
    esac

    echo "==> native_$language ($repetitions repetition(s), $OPERATIONS operations)"
    for ((repetition = 1; repetition <= repetitions; repetition++)); do
        echo "  repetition $repetition/$repetitions"
        "${command[@]}"
    done
    echo "<== native_$language passed"
}

languages=()
while (($# > 0)); do
    case "$1" in
        --languages)
            (($# >= 2)) || fail "--languages requires a value"
            parsed_languages=( $(parse_languages "$2") )
            languages+=("${parsed_languages[@]}")
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
((${#languages[@]})) || languages=("${ALL_LANGUAGES[@]}")

for language in "${languages[@]}"; do
    run_language "$language"
done

echo "Native performance tests passed: ${#languages[@]} language(s)"
