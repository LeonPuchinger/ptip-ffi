use crate::{
    codegen::CodegenOutput,
    features::{Module, ModulePath},
};

const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    // Add static assets
    let mut output = vec![
        CodegenOutput {
            path: ModulePath::new(vec!["socket.ts".to_owned()]),
            content: SOCKET_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: ModulePath::new(vec!["bridge.ts".to_owned()]),
            content: BRIDGE_IMPLEMENTATION.to_owned(),
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
