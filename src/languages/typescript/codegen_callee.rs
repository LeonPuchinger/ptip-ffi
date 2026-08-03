use std::path::PathBuf;

use crate::{codegen::CodegenOutput, features::Module};

pub fn generate_callee(modules: Vec<&Module>) -> Vec<CodegenOutput> {
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
