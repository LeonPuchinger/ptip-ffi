# Polyglot Transparent Inter-Process Foreign Function Interface (ptip-ffi)

A foreign function interface (FFI) is used to bridge code written in different programming languages.
`ptip-ffi` is an FFI that is designed around the following attributes:

- __Polyglot__: The FFI is designed to be adaptable to any language, no matter the paradigm or memory-management strategy.
- __Transparent__: The caller and callee are both completely unaware of the ffi and don't require any code changes.
- __Inter-process__: All languages stay inside their native execution enviornment, and therefore run in their own process, meaning the FFI has to work across process boundaries.

## Build

Before the tool can be used, it has to be built and installed on PATH.

```bash
cargo install --path .
```

## Usage

PTIP-FFI is invoked from the command line.
Its commands are explained below.

### Generate bindings

PTIP-FFI uses code generation to build a wrapper around foreign libraries.
To invoke the generator, the `generate` command can be used, which requires knowledge about where the input library is located, what langauge its written in, what language it should be accessible from, and where to write the output wrapper to.

```bash
cargo run -- generate \
  --library-root ./in \
  --library-language TypeScript \
  --output-directory ./out \
  --output-language Python
```

The above command calls the FFI generation pipeline for the selected input and output language.
The default library entry point is resolved automatically for supported languages.
For example, `TypeScript` defaults to `index.ts` when `--library-entry-point` is omitted.

If you need to override it explicitly:

```bash
cargo run -- generate \
  --library-root ./in \
  --library-language TypeScript \
  --library-entry-point ./in/index.ts \
  --output-directory ./out \
  --output-language TypeScript
```

The generated output is written into the `callee` and `caller` subdirectories under the configured output directory by convention.
