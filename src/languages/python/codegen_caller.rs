use std::path::PathBuf;

use crate::{
    codegen::CodegenOutput,
    features::{FunctionDefinition, Module, TypeDefinition},
};

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let mut rendered = String::from("from typing import Any\n\n");

    for module in modules {
        for function in &module.functions {
            rendered.push_str(&render_function(function));
            rendered.push('\n');
        }

        for definition in &module.types {
            rendered.push_str(&render_class(definition));
            rendered.push('\n');
        }
    }

    vec![CodegenOutput {
        path: PathBuf::from("index.py"),
        content: rendered,
    }]
}

fn render_function(function: &FunctionDefinition) -> String {
    let parameters = function
        .callable
        .positional_parameters
        .iter()
        .map(|parameter| format!("{}: Any", parameter.name))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "def {}({}) -> Any:\n    raise NotImplementedError(\"Python caller backend is not implemented yet\")\n",
        function.name, parameters
    )
}

fn render_class(definition: &TypeDefinition) -> String {
    format!(
        "class {}:\n    pass\n",
        definition.name
    )
}