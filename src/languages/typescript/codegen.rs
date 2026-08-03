use crate::{
    codegen::CodegenOutput,
    features::{Module, ModulePath},
};

// Common assets (shared between caller and callee)
const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");
// Caller assets
const CALLER_PACKAGE_JSON: &str = include_str!("./assets/caller/package.json");
const CALLER_PACKAGE_LOCK_JSON: &str = include_str!("./assets/caller/package-lock.json");
const CALLER_MAIN: &str = include_str!("./assets/caller/main.ts");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    // Add static assets
    let mut output = vec![
        CodegenOutput {
            path: ModulePath::new(vec!["ffi", "socket.ts"]),
            content: SOCKET_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: ModulePath::new(vec!["ffi", "bridge.ts"]),
            content: BRIDGE_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: ModulePath::new(vec!["ffi", "main.ts"]),
            content: CALLER_MAIN.to_owned(),
        },
        CodegenOutput {
            path: ModulePath::new(vec!["package.json"]),
            content: CALLER_PACKAGE_JSON.to_owned(),
        },
        CodegenOutput {
            path: ModulePath::new(vec!["package-lock.json"]),
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
            path: ModulePath::new(vec!["generated_callee.ts".to_owned()]),
            content: format!(
                "// Generated TypeScript code for callee module: {}",
                module.path.format(".")
            ),
        })
        .collect()
}
