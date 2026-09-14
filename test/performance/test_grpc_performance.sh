#!/usr/bin/env bash
set -Eeuo pipefail

readonly PERFORMANCE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly GRPC_ROOT="$PERFORMANCE_ROOT/alternatives/grpc"
readonly OUTPUT_ROOT="$GRPC_ROOT/output"
readonly RESULTS_ROOT="$PERFORMANCE_ROOT/results"
readonly OPERATIONS="${FFI_PERFORMANCE_OPERATIONS:-100000}"
ALL_LANGUAGES=(py ts cpp)
repetitions=1
case_index=0

usage() {
    cat <<'EOF'
Usage: test/performance/test_grpc_performance.sh [OPTIONS]

Run the gRPC performance matrix. Without filters, all caller/callee permutations run.

Options:
  --caller-languages LANGUAGES  Filter callers (py, ts, cpp; comma-separated)
  --callee-languages LANGUAGES  Filter callees (py, ts, cpp; comma-separated)
  --repetitions COUNT           Run each selected case COUNT times (default: 1)
  -h, --help                    Show this help

Environment:
  FFI_PERFORMANCE_OPERATIONS    Operations per run (default: 100000)
EOF
}

fail() { echo "error: $*" >&2; exit 2; }
parse_languages() {
    local value="$1" language; local -a parsed=()
    IFS=',' read -r -a values <<< "$value"
    for language in "${values[@]}"; do
        case "${language,,}" in
            py|python) parsed+=(py) ;; ts|typescript) parsed+=(ts) ;; cpp|c++) parsed+=(cpp) ;;
            *) fail "unknown language '$language' (expected py, ts, or cpp)" ;;
        esac
    done
    printf '%s\n' "${parsed[@]}"
}

ensure_node_dependencies() {
    npm install --prefix "$GRPC_ROOT" --silent
}

generate_bindings() {
    local output_dir="$1" caller="$2" callee="$3"
    cp "$GRPC_ROOT/hashmap.proto" "$output_dir/"
    if [[ "$caller" == py || "$callee" == py ]]; then
        python3 -m grpc_tools.protoc -I"$output_dir" --python_out="$output_dir" --grpc_python_out="$output_dir" "$output_dir/hashmap.proto"
    fi
    if [[ "$caller" == cpp || "$callee" == cpp ]]; then
        protoc -I"$output_dir" --cpp_out="$output_dir" --grpc_out="$output_dir" \
            --plugin=protoc-gen-grpc="$(command -v grpc_cpp_plugin)" "$output_dir/hashmap.proto"
    fi
}

run_case() {
    local caller="$1" callee="$2" case_name="${caller}_${callee}"
    local output_dir="$OUTPUT_ROOT/$case_name" result_file="$RESULTS_ROOT/grpc_${case_name}.csv"
    local port=$((51000 + case_index++)) server_pid client_command server_command
    local callee_directory
    case "$callee" in
        py) callee_directory=python ;;
        ts) callee_directory=typescript ;;
        cpp) callee_directory=cpp ;;
    esac
    rm -rf "$output_dir"; mkdir -p "$output_dir" "$RESULTS_ROOT"
    mkdir -p "$output_dir/input"
    cp -r "$PERFORMANCE_ROOT/input/$callee_directory" "$output_dir/input/$callee_directory"
    generate_bindings "$output_dir" "$caller" "$callee"

    if [[ "$callee" == cpp ]]; then
        g++ -std=c++20 -I"$output_dir" -I"$output_dir/callee" \
            -o "$output_dir/grpc_server" "$GRPC_ROOT/server.cpp" \
            "$output_dir/hashmap.pb.cc" "$output_dir/hashmap.grpc.pb.cc" \
            $(pkg-config --libs grpc++ protobuf) -pthread
        server_command="$output_dir/grpc_server"
    elif [[ "$callee" == py ]]; then
        server_command="python3 $GRPC_ROOT/server.py"
    else
        server_command="tsx $GRPC_ROOT/server.ts"
        ensure_node_dependencies
    fi
    if [[ "$caller" == ts ]]; then ensure_node_dependencies; fi
    if [[ "$caller" == cpp ]]; then
        g++ -std=c++20 -I"$output_dir" -I"$output_dir/caller" \
            -o "$output_dir/grpc_client" "$GRPC_ROOT/client.cpp" \
            "$output_dir/hashmap.pb.cc" "$output_dir/hashmap.grpc.pb.cc" \
            $(pkg-config --libs grpc++ protobuf) -pthread
        client_command="$output_dir/grpc_client"
    elif [[ "$caller" == py ]]; then
        client_command="python3 $GRPC_ROOT/client.py"
    else
        client_command="tsx $GRPC_ROOT/client.ts"
        ensure_node_dependencies
    fi

    echo "==> grpc_$case_name ($repetitions repetition(s), $OPERATIONS operations)"
    for ((repetition = 1; repetition <= repetitions; repetition++)); do
        echo "  repetition $repetition/$repetitions"
        setsid env FFI_GRPC_PORT="$port" GRPC_INPUT_ROOT="$output_dir" PYTHONPATH="$output_dir" sh -c "$server_command" >"$output_dir/server.log" 2>&1 &
        server_pid=$!
        for ((attempt = 0; attempt < 300; attempt++)); do
            grep -q '^ready$' "$output_dir/server.log" 2>/dev/null && break
            sleep 0.1
        done
        grep -q '^ready$' "$output_dir/server.log" || { cat "$output_dir/server.log" >&2; kill -KILL -- "-$server_pid" 2>/dev/null || true; fail "gRPC server did not start for $case_name"; }
        if [[ "$caller" == py ]]; then
            FFI_GRPC_PORT="$port" PYTHONPATH="$output_dir" python3 "$GRPC_ROOT/client.py" "$OPERATIONS" "$result_file"
        elif [[ "$caller" == ts ]]; then
            FFI_GRPC_PORT="$port" tsx "$GRPC_ROOT/client.ts" "$OPERATIONS" "$result_file"
        else
            FFI_GRPC_PORT="$port" "$output_dir/grpc_client" "$OPERATIONS" "$result_file"
        fi
        kill -KILL -- "-$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true
    done
    echo "<== grpc_$case_name passed"
}

caller_languages=() callee_languages=()
while (($# > 0)); do
    case "$1" in
        --caller-languages|--callee-languages)
            (($# >= 2)) || fail "$1 requires a value"; parsed_languages=( $(parse_languages "$2") )
            [[ "$1" == --caller-languages ]] && caller_languages+=("${parsed_languages[@]}") || callee_languages+=("${parsed_languages[@]}"); shift 2 ;;
        --repetitions) (($# >= 2)) || fail "--repetitions requires a value"; [[ "$2" =~ ^[1-9][0-9]*$ ]] || fail "--repetitions must be positive"; repetitions="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) fail "unknown option '$1'" ;;
    esac
done
[[ "$OPERATIONS" =~ ^[1-9][0-9]*$ ]] || fail "FFI_PERFORMANCE_OPERATIONS must be positive"
((${#caller_languages[@]})) || caller_languages=("${ALL_LANGUAGES[@]}")
((${#callee_languages[@]})) || callee_languages=("${ALL_LANGUAGES[@]}")
for caller in "${caller_languages[@]}"; do for callee in "${callee_languages[@]}"; do run_case "$caller" "$callee"; done; done
echo "gRPC performance matrix passed: ${#caller_languages[@]} caller(s) x ${#callee_languages[@]} callee(s)"
