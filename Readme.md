# Polyglot Transparent Inter-Process Foreign Function Interface (ptip-ffi)

A foreign function interface (FFI) is used to bridge code written in different programming languages.
`ptip-ffi` is an FFI that is designed around the following attributes:

- __Polyglot__: The FFI is designed to be adaptable to any language, no matter the paradigm or memory-management strategy.
- __Transparent__: The caller and callee are both completely unaware of the ffi and don't require any code changes.
- __Inter-process__: All languages stay inside their native execution enviornment, and therefore run in their own process, meaning the FFI has to work across process boundaries.
