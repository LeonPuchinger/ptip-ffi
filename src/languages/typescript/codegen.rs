use crate::{
    codegen::CodegenOutput,
    features::{Module, ModulePath},
};

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
