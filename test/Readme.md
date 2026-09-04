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

From the repository root:

```bash
test/performance/test_ffi_performance
```

By default, all nine caller/callee combinations run once with `100000` operations. Filter the matrix with the same language options as the smoke test and use `--repetitions` to repeat each selected case:

```bash
test/performance/test_ffi_performance \
  --caller-languages py,ts \
  --callee-languages cpp \
  --repetitions 5
```

Set `FFI_PERFORMANCE_OPERATIONS` to change the operations per measurement without changing the matrix options:

```bash
FFI_PERFORMANCE_OPERATIONS=1000000 test/performance/test_ffi_performance
```

Each case starts from a clean generated directory under `test/performance/output/<caller>_<callee>`. Measurement files are written to `test/performance/results/ffi_<caller>_<callee>.csv` with columns `timestamp,operations,key_alphabet,max_insert_value,elapsed_ms`.

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
