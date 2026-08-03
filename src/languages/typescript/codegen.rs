use std::path::PathBuf;

use crate::{codegen::CodegenOutput, features::Module};

// Common assets (shared between caller and callee)
const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");
// Caller assets
const CALLER_PACKAGE_JSON: &str = include_str!("./assets/caller/package.json");
const CALLER_PACKAGE_LOCK_JSON: &str = include_str!("./assets/caller/package-lock.json");
const CALLER_MAIN: &str = include_str!("./assets/caller/main.ts");
const CALLER_INDEX: &str = include_str!("./assets/caller/index.ts");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    // Add static assets
    let mut output = vec![
        CodegenOutput {
            path: PathBuf::from("ffi/socket.ts"),
            content: SOCKET_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/bridge.ts"),
            content: BRIDGE_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/main.ts"),
            content: CALLER_MAIN.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("package.json"),
            content: CALLER_PACKAGE_JSON.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("package-lock.json"),
            content: CALLER_PACKAGE_LOCK_JSON.to_owned(),
        },
    ];
    output
}

pub fn generate_callee(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    // dummy implementation
    modules
        .into_iter()
        .map(|module| CodegenOutput {
            path: PathBuf::from("generated_callee.ts"),
            content: format!(
                "// Generated TypeScript code for callee module: {}",
                module.path.format(".")
            ),
        })
        .collect()
}
