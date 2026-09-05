# Tests

This directory contains automated tests for `ptip-ffi`, which can be divided into different test formats, e.g. smoke tests or performance tests.
The point of this document is to describe the reason behind each format and how to invoke the tests.

## Smoke Tests

The somoke tests invokes a short test of every combination of caller/callee languages to test basic functionality of `ptip-ffi` itself, as well as it codegen output.

For each combination of caller/callee languages (testcase), `test/smoke/test.sh` performs the following steps:

1. Deletes any remaining files of previous invocations of the same testcase.
2. Invokes the code generator of PTIP-FFI using up-to-date sources, writing generated files to `test/smoke/output/<caller>_<callee>`.
3. Copies a testing script (referred to as the usage program) to the output directory of the code generator. The usage script implements the smoke test by invocating the caller side of the generated output.
4. Executes the 

### Run the Matrix

From the repository root:

```bash
test/smoke/test.sh
```

Without options, all caller/callee combinations are tested.

### Filter the Matrix

Use `--caller-languages` and `--callee-languages` to select languages. Values may be comma-separated or supplied more than once.
The languages are abbreviated, e.g. `py`, `ts`, and `cpp`.

```bash
# Test only TypeScript caller and Python callee
test/smoke/test.sh --caller-languages ts --callee-languages py

# Test Python and TypeScript callers against Python and C++ callees
test/smoke/test.sh \
  --caller-languages py,ts \
  --callee-languages py,cpp
```

Use `test/smoke/test.sh --help` to display the available options.

## Performance Tests

The performance test measures HashMap operations through every caller/callee permutation and writes results using the same CSV schema as the native baselines.
The performance test measures how long a set number of HashMap operations takes when performed over the FFI.

To invoke the FFI performance test, the following command can be invoked from the repository root:

```bash
test/performance/test_ffi_performance.sh
```

By default, all nine caller/callee combinations run once with `100000` operations.
The matrix can be filtered with the same language options as the smoke test and the `--repetitions` flag can be used to control how often each selected case is run:

```bash
test/performance/test_ffi_performance \
  --caller-languages py,ts \
  --callee-languages cpp \
  --repetitions 5
```

The `FFI_PERFORMANCE_OPERATIONS` environment variable can be used to change the HashMap operations per measurement without changing the matrix options:

```bash
FFI_PERFORMANCE_OPERATIONS=1000000 test/performance/test_ffi_performance
```

Each case starts from a cleanly generated directory under `test/performance/output/<caller>_<callee>`.
Measurement files are written to `test/performance/results/ffi_<caller>_<callee>.csv` with columns `timestamp,operations,key_alphabet,max_insert_value,elapsed_ms`.

As a comparison, the performance test includes a way to measure the native performance as well as the performance of an alternative FFI solution.
In the native performance test, the same HashMap-based scenario is invoked, but not over the FFI.
Instead, each language only calls it own, native implementation.
To run the native baseline performance tests, use:

```bash
test/performance/test_native_performance.sh
```

Without options, all three native implementations run once. Use `--languages` to select languages and `--repetitions` to repeat each selected native test:

```bash
test/performance/test_native_performance.sh \
  --languages py,cpp \
  --repetitions 5
```

The native runner uses the same `FFI_PERFORMANCE_OPERATIONS` environment variable as the FFI runner.
Native measurements are appended to `test/performance/results/native_<language>.csv`.

For the performance test of the alternate FFI solution, gRPC is used.
To be able to run the gRPC performance test, gRPC needs to be setup on the test runner with support for python, cpp, and typescript installed.
It it also invoked in a similar way to the FFI performance test, as shown in the following:

Run the complete matrix:

```bash
test/performance/test_grpc_performance.sh
```

The runner accepts the same caller/callee filters and repetition option as the FFI runner:

```bash
test/performance/test_grpc_performance.sh \
  --caller-languages py,cpp \
  --callee-languages ts \
  --repetitions 5
```

The workload is controlled with `FFI_PERFORMANCE_OPERATIONS`, for example:

```bash
FFI_PERFORMANCE_OPERATIONS=1000000 test/performance/test_grpc_performance.sh
```

Generated gRPC artifacts are placed under `test/performance/alternatives/grpc/output/<caller>_<callee>`.
Results are appended to `test/performance/results/grpc_<caller>_<callee>.csv` using the common performance schema.

## Unit Tests

The functionality of `ptip-ffi` itself (e.g. its parser or code generator) is tested via unit tests implemented in Rust.
These tests can be invoked from the repository root via `cargo test`.
Per Rust's conventions, the unit-test implementations are kept within the same module/file that is supposed to be tested.

## Asset Tests

The asset files used during code generation can be divided into static and dynamic assets.
Dynamic assets are template files that the templating engine of the code generator inserts dynamically generated code snippets into.
Before code generation, dynamic assets are not runnable as they are not semantically and syntactically complete.
On the other hand, static assets are copied to the output directory without modification during codegen and are therefore runnable and testable.
To reproducibly test these assets, they are tested in a container-based test setup specific to each tested language.
Asset tests can be found in `src/languages/<language-to-test>/assets/tests`.
Asset tests can be invoked via `make test`, which automates the container setup and test invocation.
The only requirements to execute the asset tests are `docker` and `make` to be installed on the test runner.
