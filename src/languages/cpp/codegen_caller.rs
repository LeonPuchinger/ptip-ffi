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
            path: PathBuf::from("ffi/main.hpp"),
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
        let body = render_module_stubs(engine, module);
        if module.path.segments.is_empty() {
            out.push(body);
        } else {
            out.push(format!("namespace {} {{\n{}\n}}", module.path.format("::"), indent(&body, 2)));
        }
    }
    out.join("\n\n")
}

fn render_module_stubs(engine: &TemplateEngine, module: &Module) -> String {
    let mut out = Vec::new();
    for definition in &module.types {
        out.push(render_type_stub(engine, definition));
    }
    for function in &module.functions {
        out.push(render_function_stub(engine, function));
    }
    out.join("\n\n")
}

fn render_function_stub(_engine: &TemplateEngine, function: &FunctionDefinition) -> String {
    let type_parameters = render_type_parameters(&function.callable.type_parameters);
    let return_type = render_type_name(&function.callable.return_type);
    let parameters = render_parameters(&function.callable.positional_parameters);

    let mut out = String::new();
    out.push_str(&type_parameters);
    out.push_str(&return_type);
    out.push(' ');
    out.push_str(&function.name);
    out.push('(');
    out.push_str(&parameters);
    out.push_str(") {\n");

    for parameter in &function.callable.positional_parameters {
        let local_name = parameter_name_binding(&parameter.name);
        out.push_str("    const ptip_ffi::Parameter ");
        out.push_str(&local_name);
        out.push_str(" = ");
        out.push_str(&render_parameter_value_cpp(&parameter.r#type, &parameter.name));
        out.push_str(";\n");
    }

    out.push_str("    const std::string return_sink = ptip_ffi::generate_uuid();\n");
    out.push_str("    auto& bridge = ptip_ffi::establishBridge();\n");
    out.push_str("    bridge.send_message(\"C\\n\" + ptip_ffi::serialize_invocation_path(\"\", \"");
    out.push_str(&function.name);
    out.push_str("\") + \"\\n\" + return_sink");
    for parameter in &function.callable.positional_parameters {
        out.push_str(" + \"\\n\" + ptip_ffi::encode_parameter_line(");
        out.push_str(&parameter_name_binding(&parameter.name));
        out.push_str(")");
    }
    out.push_str(");\n");
    out.push_str("    const std::string response = bridge.next_message();\n");
    out.push_str("    if (response.empty()) {\n        throw std::runtime_error(\"No response received from the bridge\");\n    }\n");
    out.push_str("    const auto lines = ptip_ffi::split_message_lines(response);\n");
    out.push_str("    if (lines.size() < 3 || lines[0] != \"S\" || lines[1] != return_sink) {\n        if (lines.size() >= 2 && lines[0] == \"E\") {\n            throw std::runtime_error(\"FFI call failed\");\n        }\n        throw std::runtime_error(\"Unexpected bridge response\");\n    }\n");
    out.push_str("    const auto decoded_value = ptip_ffi::decode_parameter_line(lines[2]);\n");
    out.push_str(&render_bridge_return_statement(&function.callable.return_type, "decoded_value"));
    out.push_str("\n}");
    out
}

fn render_type_stub(engine: &TemplateEngine, definition: &TypeDefinition) -> String {
    let type_parameters = render_type_parameters(&definition.type_parameters);
    let mut members = Vec::new();
    members.push("std::string uuid;".to_string());
    if let Some(constructor) = &definition.default_constructor {
        let mut ctor = String::new();
        ctor.push_str(&definition.name);
        ctor.push('(');
        ctor.push_str(&render_parameters(&constructor.positional_parameters));
        ctor.push_str(") {\n");
        ctor.push_str("    const std::string return_sink = ptip_ffi::generate_uuid();\n");
        ctor.push_str("    auto& bridge = ptip_ffi::establishBridge();\n");
        ctor.push_str("    bridge.send_message(\"C\\n\" + ptip_ffi::serialize_invocation_path(\"\", \"");
        ctor.push_str(&definition.name);
        ctor.push_str("\") + \"\\n\" + return_sink");
        for parameter in &constructor.positional_parameters {
            ctor.push_str(" + \"\\n\" + ptip_ffi::encode_parameter_line(");
            ctor.push_str(&render_parameter_value_cpp(&parameter.r#type, &parameter.name));
            ctor.push_str(")");
        }
        ctor.push_str(");\n");
        ctor.push_str("    const std::string response = bridge.next_message();\n");
        ctor.push_str("    if (response.empty()) {\n        throw std::runtime_error(\"No response received from the bridge\");\n    }\n");
        ctor.push_str("    const auto lines = ptip_ffi::split_message_lines(response);\n");
        ctor.push_str("    if (lines.size() < 3 || lines[0] != \"S\" || lines[1] != return_sink) {\n        throw std::runtime_error(\"Unexpected bridge response\");\n    }\n");
        ctor.push_str("    const auto value = ptip_ffi::decode_parameter_line(lines[2]);\n");
        ctor.push_str("    if (value.kind != ptip_ffi::ParameterKind::Reference) {\n        throw std::runtime_error(\"Constructor did not return a reference\");\n    }\n");
        ctor.push_str("    this->uuid = value.value;\n");
        ctor.push_str("  }");
        members.push(ctor);
    } else {
        let mut ctor = String::new();
        ctor.push_str(&definition.name);
        ctor.push_str("() {\n");
        ctor.push_str("    const std::string return_sink = ptip_ffi::generate_uuid();\n");
        ctor.push_str("    auto& bridge = ptip_ffi::establishBridge();\n");
        ctor.push_str("    bridge.send_message(\"C\\n\" + ptip_ffi::serialize_invocation_path(\"\", \"");
        ctor.push_str(&definition.name);
        ctor.push_str("\") + \"\\n\" + return_sink);\n");
        ctor.push_str("    const std::string response = bridge.next_message();\n");
        ctor.push_str("    if (response.empty()) {\n        throw std::runtime_error(\"No response received from the bridge\");\n    }\n");
        ctor.push_str("    const auto lines = ptip_ffi::split_message_lines(response);\n");
        ctor.push_str("    if (lines.size() < 3 || lines[0] != \"S\" || lines[1] != return_sink) {\n        throw std::runtime_error(\"Unexpected bridge response\");\n    }\n");
        ctor.push_str("    const auto value = ptip_ffi::decode_parameter_line(lines[2]);\n");
        ctor.push_str("    if (value.kind != ptip_ffi::ParameterKind::Reference) {\n        throw std::runtime_error(\"Default constructor did not return a reference\");\n    }\n");
        ctor.push_str("    this->uuid = value.value;\n");
        ctor.push_str("  }");
        members.push(ctor);
    }
    for constructor in &definition.named_constructors {
        members.push(format!(
            "static {} {}({}) {{ return {}; }}",
            render_type_name(&constructor.callable.return_type),
            constructor.name,
            render_parameters(&constructor.callable.positional_parameters),
            constructor.name
        ));
    }
    for property in &definition.properties {
        members.push(format!("{} {};", render_type_name(&property.1), property.0));
    }
    for method in &definition.methods {
        members.push(render_method_stub(engine, method));
    }
    for method in &definition.static_methods {
        members.push(render_method_stub(engine, method));
    }
    members.push(format!(
        "~{}() {{\n    if (!this->uuid.empty()) {{\n        auto& bridge = ptip_ffi::establishBridge();\n        bridge.send_message(\"D\\n\" + this->uuid);\n    }}\n}}",
        definition.name
    ));
    members.push(format!(
        "static {} __fromReference(const std::string& value) {{\n    {} instance;\n    instance.uuid = value;\n    return instance;\n}}",
        definition.name,
        definition.name
    ));

    if members.is_empty() {
        format!("{}struct {} {{}};", type_parameters, definition.name)
    } else {
        format!(
            "{}struct {} {{\n  {}\n}};",
            type_parameters,
            definition.name,
            members.join("\n  ")
        )
    }
}

fn parameter_name_binding(name: &str) -> String {
    let reserved = [
        "value", "key", "index", "str", "point", "response", "lines", "bridge", "return_sink",
        "decoded_value",
    ];
    if reserved.contains(&name) {
        format!("{name}_param")
    } else {
        name.to_string()
    }
}

fn render_method_stub(_engine: &TemplateEngine, method: &Method) -> String {
    let parameters = render_parameters(&method.callable.positional_parameters);

    let mut out = String::new();
    if method.r#static {
        out.push_str("static ");
    }
    out.push_str(&render_type_name(&method.callable.return_type));
    out.push(' ');
    out.push_str(&method.name);
    out.push('(');
    out.push_str(&parameters);
    out.push_str(")");
    if !method.r#static {
        out.push_str(" const");
    }
    out.push_str(" {\n");

    for parameter in &method.callable.positional_parameters {
        let local_name = parameter_name_binding(&parameter.name);
        out.push_str("    const ptip_ffi::Parameter ");
        out.push_str(&local_name);
        out.push_str(" = ");
        out.push_str(&render_parameter_value_cpp(&parameter.r#type, &parameter.name));
        out.push_str(";\n");
    }

    out.push_str("    const std::string return_sink = ptip_ffi::generate_uuid();\n");
    out.push_str("    auto& bridge = ptip_ffi::establishBridge();\n");
    out.push_str("    bridge.send_message(\"M\\n\" + this->uuid + \"\\n\" + ptip_ffi::encode_base64_no_pad_utf8(\"");
    out.push_str(&method.name);
    out.push_str("\") + \"\\n\" + return_sink");
    for parameter in &method.callable.positional_parameters {
        out.push_str(" + \"\\n\" + ptip_ffi::encode_parameter_line(");
        out.push_str(&parameter_name_binding(&parameter.name));
        out.push_str(")");
    }
    out.push_str(");\n");
    out.push_str("    const std::string response = bridge.next_message();\n");
    out.push_str("    if (response.empty()) {\n        throw std::runtime_error(\"No response received from the bridge\");\n    }\n");
    out.push_str("    const auto lines = ptip_ffi::split_message_lines(response);\n");
    out.push_str("    if (lines.size() < 3 || lines[0] != \"S\" || lines[1] != return_sink) {\n        if (lines.size() >= 2 && lines[0] == \"E\") {\n            throw std::runtime_error(\"FFI call failed\");\n        }\n        throw std::runtime_error(\"Unexpected bridge response\");\n    }\n");
    out.push_str("    const auto decoded_value = ptip_ffi::decode_parameter_line(lines[2]);\n");
    out.push_str(&render_bridge_return_statement(&method.callable.return_type, "decoded_value"));
    out.push_str("\n}");
    out
}

fn render_parameter_value_cpp(r#type: &Type, name: &str) -> String {
    match r#type {
        Type::Primitive(crate::features::PrimitiveType::Number) => {
            format!("ptip_ffi::Parameter{{ptip_ffi::ParameterKind::Float, std::to_string({})}}", name)
        }
        Type::Primitive(crate::features::PrimitiveType::String) => {
            format!("ptip_ffi::Parameter{{ptip_ffi::ParameterKind::String, {}}}", name)
        }
        Type::Primitive(crate::features::PrimitiveType::Boolean) => {
            format!("ptip_ffi::Parameter{{ptip_ffi::ParameterKind::Boolean, {} ? \"1\" : \"0\"}}", name)
        }
        Type::Composite(_) => {
            format!("ptip_ffi::encode_value({})", name)
        }
        Type::Pointer(inner) => match inner.as_ref() {
            Type::Composite(_) => format!("ptip_ffi::encode_value(*{})", name),
            Type::Primitive(crate::features::PrimitiveType::Number) => format!(
                "ptip_ffi::Parameter{{ptip_ffi::ParameterKind::Float, std::to_string(*{})}}",
                name
            ),
            Type::Primitive(crate::features::PrimitiveType::Boolean) => format!(
                "ptip_ffi::Parameter{{ptip_ffi::ParameterKind::Boolean, *{} ? \"1\" : \"0\"}}",
                name
            ),
            Type::Primitive(crate::features::PrimitiveType::String) => format!(
                "ptip_ffi::Parameter{{ptip_ffi::ParameterKind::String, *{}}}",
                name
            ),
            _ => format!("ptip_ffi::encode_value(*{})", name),
        },
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => {
            format!("ptip_ffi::Parameter{{ptip_ffi::ParameterKind::String, std::string(\"\")}}")
        }
    }
}

fn render_bridge_return_statement(r#type: &Type, value_name: &str) -> String {
    match r#type {
        Type::Primitive(crate::features::PrimitiveType::Number) => {
            format!(
                "    if ({value_name}.kind == ptip_ffi::ParameterKind::Float || {value_name}.kind == ptip_ffi::ParameterKind::Integer) {{\n        return std::stod({value_name}.value);\n    }}\n    throw std::runtime_error(\"Unexpected return type\");"
            )
        }
        Type::Primitive(crate::features::PrimitiveType::String) => {
            format!(
                "    if ({value_name}.kind == ptip_ffi::ParameterKind::String) {{\n        return {value_name}.value;\n    }}\n    throw std::runtime_error(\"Unexpected return type\");"
            )
        }
        Type::Primitive(crate::features::PrimitiveType::Boolean) => {
            format!(
                "    if ({value_name}.kind == ptip_ffi::ParameterKind::Boolean) {{\n        return {value_name}.value == \"1\";\n    }}\n    throw std::runtime_error(\"Unexpected return type\");"
            )
        }
        Type::Dynamic => "".to_string(),
        Type::Composite(path) => {
            let type_name = if path.module_path.segments.is_empty() {
                path.name.clone()
            } else {
                format!("{}::{}", path.module_path.format("::"), path.name)
            };
            let is_template_parameter = path.module_path.segments.is_empty()
                && path.type_arguments.is_empty()
                && path.name.len() == 1
                && path.name.chars().next().is_some_and(|ch| ch.is_ascii_uppercase());
            if is_template_parameter {
                format!(
                    "    return ptip_ffi::decode_value<{type_name}>({value_name});",
                    value_name = value_name,
                    type_name = type_name,
                )
            } else {
                format!(
                    "    if ({value_name}.kind == ptip_ffi::ParameterKind::Reference) {{\n        return ptip_ffi::decode_value<{type_name}>({value_name});\n    }}\n    throw std::runtime_error(\"Unexpected return type\");",
                    value_name = value_name,
                    type_name = type_name,
                )
            }
        }
        Type::Pointer(inner) => render_bridge_return_statement(inner, value_name),
        Type::Array(_) | Type::Tuple(_) => {
            "    throw std::runtime_error(\"Unsupported return type\");".to_string()
        }
    }
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
        Type::Dynamic => "void".to_string(),
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
        Type::Pointer(inner) => format!("{}*", render_type_name(inner)),
        Type::Array(inner) => format!("std::vector<{}>", render_type_name(inner)),
        Type::Tuple(elements) => format!(
            "std::tuple<{}>",
            elements.iter().map(render_type_name).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn render_type_parameters(parameters: &[TypeParameter]) -> String {
    if parameters.is_empty() {
        return String::new();
    }
    format!(
        "template <{}>\n",
        parameters
            .iter()
            .map(|parameter| format!("typename {}", parameter.name))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn indent(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
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

        let main = outputs
            .iter()
            .find(|output| output.path == Path::new("ffi/main.hpp"))
            .expect("ffi/main.hpp should be generated");

        assert!(index.content.contains("#include \"ffi/main.hpp\""));
        assert!(index.content.contains("double add(double x, double y)"));
        assert!(index.content.contains("establishBridge();"));
        assert!(index.content.contains("bridge.send_message("));
        assert!(index.content.contains("struct Point"));
        assert!(index.content.contains("double distance() const"));
        assert!(main.content.contains("establishBridge"));
    }

    #[test]
    fn generate_caller_emits_drop_destructor_for_managed_types() {
        let module = Module {
            path: ModulePath::empty(),
            functions: Vec::new(),
            types: vec![TypeDefinition {
                name: "Point".to_string(),
                properties: vec![],
                default_constructor: None,
                named_constructors: Vec::new(),
                methods: Vec::new(),
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

        assert!(index.content.contains("~Point()"));
        assert!(index.content.contains("bridge.send_message(\"D\\n\" + this->uuid)"));
    }
}
