use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{FunctionDefinition, Method, Module, Type, TypeDefinition, TypeParameter, ValueParameter},
};

const CALLER_BRIDGE: &str = include_str!("./assets/bridge.hpp");
const CALLER_SOCKET: &str = include_str!("./assets/socket.hpp");
const CALLER_MAIN: &str = include_str!("./assets/caller_main.hpp");
const CALLER_STUB: &str = include_str!("./assets/caller_stub.hpp");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");
    let generated = render_caller_stubs(&engine, &modules);

    vec![
        CodegenOutput {
            path: PathBuf::from("ffi/bridge.hpp"),
            content: CALLER_BRIDGE.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/socket.hpp"),
            content: CALLER_SOCKET.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("main.hpp"),
            content: CALLER_MAIN.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("index.hpp"),
            content: engine.render(
                CALLER_STUB,
                &crate::map! {
                    "NAME" => "ptip_ffi_generated",
                    "STUBS" => generated.as_str(),
                },
                false,
            ),
        },
    ]
}

fn render_caller_stubs(engine: &TemplateEngine, modules: &[&Module]) -> String {
    let mut out = Vec::new();
    for module in modules {
        for function in &module.functions {
            out.push(render_function_stub(engine, function));
        }
        for definition in &module.types {
            out.push(render_type_stub(engine, definition));
        }
    }
    out.join("\n\n")
}

fn render_function_stub(_engine: &TemplateEngine, function: &FunctionDefinition) -> String {
    format!(
        "inline {} {}({}) {{ return {}; }}",
        render_type_name(&function.callable.return_type),
        function.name,
        render_parameters(&function.callable.positional_parameters),
        function.name
    )
}

fn render_type_stub(engine: &TemplateEngine, definition: &TypeDefinition) -> String {
    let mut members = Vec::new();
    for property in &definition.properties {
        members.push(format!("{} {};", render_type_name(&property.1), property.0));
    }
    for method in &definition.methods {
        members.push(render_method_stub(engine, method));
    }
    let class_or_struct = if definition.properties.is_empty() && definition.methods.is_empty() {
        "struct"
    } else {
        "struct"
    };
    format!(
        "{} {} {{\n  {}\n}};",
        class_or_struct,
        definition.name,
        members.join("\n  ")
    )
}

fn render_method_stub(_engine: &TemplateEngine, method: &Method) -> String {
    format!(
        "{} {}({}) const;",
        render_type_name(&method.callable.return_type),
        method.name,
        render_parameters(&method.callable.positional_parameters)
    )
}

fn render_parameters(parameters: &[ValueParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("{} {}", render_type_name(&parameter.r#type), parameter.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_type_name(r#type: &Type) -> String {
    match r#type {
        Type::Primitive(crate::features::PrimitiveType::Number) => "double".to_string(),
        Type::Primitive(crate::features::PrimitiveType::String) => "std::string".to_string(),
        Type::Primitive(crate::features::PrimitiveType::Boolean) => "bool".to_string(),
        Type::Composite(path) => {
            let mut name = path.name.clone();
            if !path.type_arguments.is_empty() {
                let args = path
                    .type_arguments
                    .iter()
                    .map(render_type_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                name = format!("{}<{}>", name, args);
            }
            if !path.module_path.segments.is_empty() {
                format!("{}::{}", path.module_path.format("::"), name)
            } else {
                name
            }
        }
        Type::Array(inner) => format!("std::vector<{}>", render_type_name(inner)),
        Type::Tuple(_) => "std::tuple<>".to_string(),
        Type::Dynamic => "auto".to_string(),
    }
}

fn render_type_parameters(_parameters: &[TypeParameter]) -> String {
    String::new()
}
