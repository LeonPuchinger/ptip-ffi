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
    let return_type = render_type_name(&function.callable.return_type);
    let parameters = render_parameters(&function.callable.positional_parameters);

    if parameters.is_empty() {
        format!("inline {} {}() {{ return ::{}(); }}", return_type, function.name, function.name)
    } else {
        format!(
            "inline {} {}({}) {{ return ::{}({}); }}",
            return_type,
            function.name,
            parameters,
            function.name,
            function
                .callable
                .positional_parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn render_type_stub(engine: &TemplateEngine, definition: &TypeDefinition) -> String {
    let mut members = Vec::new();
    for property in &definition.properties {
        members.push(format!("{} {};", render_type_name(&property.1), property.0));
    }
    for method in &definition.methods {
        members.push(render_method_stub(engine, method));
    }

    if members.is_empty() {
        format!("struct {} {{\n  // empty type\n}};", definition.name)
    } else {
        format!(
            "struct {} {{\n  {}\n}};",
            definition.name,
            members.join("\n  ")
        )
    }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::generate_caller;
    use crate::features::{
        AnonymousCallable, FunctionDefinition, Method, Module, ModulePath, PrimitiveType, Type,
        TypeDefinition, ValueParameter,
    };

    #[test]
    fn generate_caller_renders_function_and_type_stubs() {
        let module = Module {
            path: ModulePath::empty(),
            functions: vec![FunctionDefinition {
                name: "add".to_string(),
                callable: AnonymousCallable {
                    positional_parameters: vec![
                        ValueParameter {
                            name: "x".to_string(),
                            r#type: Type::Primitive(PrimitiveType::Number),
                            required: true,
                            variadic: false,
                            nullable: false,
                        },
                        ValueParameter {
                            name: "y".to_string(),
                            r#type: Type::Primitive(PrimitiveType::Number),
                            required: true,
                            variadic: false,
                            nullable: false,
                        },
                    ],
                    named_parameters: Vec::new(),
                    return_type: Type::Primitive(PrimitiveType::Number),
                    type_parameters: Vec::new(),
                },
            }],
            types: vec![TypeDefinition {
                name: "Point".to_string(),
                properties: vec![
                    ("x".to_string(), Type::Primitive(PrimitiveType::Number)),
                    ("y".to_string(), Type::Primitive(PrimitiveType::Number)),
                ],
                default_constructor: None,
                named_constructors: Vec::new(),
                methods: vec![Method {
                    name: "distance".to_string(),
                    r#static: false,
                    callable: AnonymousCallable {
                        positional_parameters: Vec::new(),
                        named_parameters: Vec::new(),
                        return_type: Type::Primitive(PrimitiveType::Number),
                        type_parameters: Vec::new(),
                    },
                }],
                static_methods: Vec::new(),
                type_parameters: Vec::new(),
                implements: Vec::new(),
            }],
        };

        let outputs = generate_caller(vec![&module]);
        let index = outputs
            .iter()
            .find(|output| output.path == Path::new("index.hpp"))
            .expect("index.hpp should be generated");

        assert!(index.content.contains("inline double add(double x, double y)"));
        assert!(index.content.contains("struct Point"));
        assert!(index.content.contains("double distance() const"));
    }
}
