use std::path::PathBuf;

use crate::{
    codegen::{template::TemplateEngine, CodegenOutput},
    features::{FunctionDefinition, Module, TypeDefinition, ValueParameter},
    map,
};

const CALLER_INIT: &str = include_str!("./assets/caller/__init__.py");
const FFI_BRIDGE: &str = include_str!("./assets/bridge.py");
const FFI_SOCKET: &str = include_str!("./assets/socket.py");
const CALLER_INDEX_TEMPLATE: &str = include_str!("./assets/caller/index.py");
const CALLER_FUNCTION_TEMPLATE: &str = include_str!("./assets/caller/function_stub.py");
const CALLER_CLASS_TEMPLATE: &str = include_str!("./assets/caller/class_stub.py");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");

    let stubs = render_caller_stubs(&engine, &modules);
    let rendered_index = engine.render(
        CALLER_INDEX_TEMPLATE,
        &map! {
            "STUBS" => stubs.as_str(),
        },
        true,
    );

    vec![
        CodegenOutput {
            path: PathBuf::from("__init__.py"),
            content: CALLER_INIT.to_string(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/bridge.py"),
            content: FFI_BRIDGE.to_string(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/socket.py"),
            content: FFI_SOCKET.to_string(),
        },
        CodegenOutput {
            path: PathBuf::from("index.py"),
            content: rendered_index,
        },
    ]
}

fn render_caller_stubs(engine: &TemplateEngine, modules: &[&Module]) -> String {
    let mut stubs = Vec::new();
    for module in modules {
        for function in &module.functions {
            stubs.push(render_function_stub(engine, function));
        }
        for definition in &module.types {
            stubs.push(render_class_stub(engine, definition));
        }
    }
    stubs.join("\n\n")
}

fn render_function_stub(engine: &TemplateEngine, function: &FunctionDefinition) -> String {
    let signature = render_parameters(&function.callable.positional_parameters);
    let positional_arguments = render_call_arguments(&function.callable.positional_parameters);
    let named_arguments = render_named_arguments(&function.callable.positional_parameters);
    engine.render(
        CALLER_FUNCTION_TEMPLATE,
        &map! {
            "NAME" => function.name.as_str(),
            "SIGNATURE" => signature.as_str(),
            "POSITIONAL_ARGUMENTS" => positional_arguments.as_str(),
            "NAMED_ARGUMENTS" => named_arguments.as_str(),
        },
        false,
    )
}

fn render_class_stub(engine: &TemplateEngine, definition: &TypeDefinition) -> String {
    let positional_arguments = "python_to_parameter(value) for value in args".to_string();
    let named_arguments = "key: python_to_parameter(value) for key, value in kwargs.items()".to_string();
    engine.render(
        CALLER_CLASS_TEMPLATE,
        &map! {
            "NAME" => definition.name.as_str(),
            "POSITIONAL_ARGUMENTS" => positional_arguments.as_str(),
            "KWARG_ARGUMENTS" => named_arguments.as_str(),
        },
        false,
    )
}

fn render_parameters(parameters: &[ValueParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            if parameter.variadic {
                format!("*{}: Any", parameter.name)
            } else if parameter.required {
                format!("{}: Any", parameter.name)
            } else {
                format!("{}: Any = None", parameter.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_call_arguments(parameters: &[ValueParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            if parameter.variadic {
                format!("*{}", parameter.name)
            } else {
                parameter.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_named_arguments(parameters: &[ValueParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("\"{}\": python_to_parameter({})", parameter.name, parameter.name))
        .collect::<Vec<_>>()
        .join(", ")
}
