use crate::{
    codegen::CodegenOutput,
    features::{Module, ModulePath},
};

const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");

pub fn generate(module: &Module) -> CodegenOutput {
    // dummy implementation
    CodegenOutput {
        path: ModulePath::new(vec!["generated.ts".to_owned()]),
        content: format!(
            "// Generated TypeScript code for module: {}",
            module.path.format(".")
        ),
    }
}
