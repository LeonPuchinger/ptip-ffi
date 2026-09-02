# Tests

This directory contains automated tests for `ptip-ffi`.

## Smoke Tests

The smoke test exercises every caller/callee permutation of the supported languages of PTIP-FFI.

For each selected case, `test/smoke/test.sh` performs the following steps:

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
