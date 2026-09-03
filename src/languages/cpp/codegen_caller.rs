use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{
        FunctionDefinition, Method, Module, Type, TypeDefinition, TypeParameter, ValueParameter,
    },
};

const CALLER_BRIDGE: &str = include_str!("./assets/bridge.hpp");
const CALLER_SOCKET: &str = include_str!("./assets/socket.hpp");
const CALLER_MAIN: &str = include_str!("./assets/caller_main.hpp");
const CALLER_STUB: &str = include_str!("./assets/caller_stub.hpp");
const CALLER_FUNCTION_STUB: &str = include_str!("./assets/caller/function_stub.hpp");
const CALLER_TYPE_STUB: &str = include_str!("./assets/caller/type_stub.hpp");
const CALLER_METHOD_STUB: &str = include_str!("./assets/caller/method_stub.hpp");
const CALLER_CONSTRUCTOR_STUB: &str = include_str!("./assets/caller/constructor_stub.hpp");
const CALLER_BRIDGE_CALL_BODY: &str = include_str!("./assets/caller/bridge_call_body.hpp");
const CALLER_ERROR_HANDLING_FUNCTION: &str = include_str!("./assets/caller/error_handling_function.hpp");
const CALLER_ERROR_HANDLING_CONSTRUCTOR: &str = include_str!("./assets/caller/error_handling_constructor.hpp");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");
    let generated = render_caller_stubs(&engine, &modules);
    let forward_declarations = render_forward_declarations(&modules);

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
                    "FORWARD_DECLARATIONS" => forward_declarations.as_str(),
                    "STUBS" => generated.as_str(),
                },
                false,
            ),
        },
    ]
}

fn render_forward_declarations(modules: &[&Module]) -> String {
    let mut declarations = BTreeMap::new();
    for module in modules {
        for definition in &module.types {
            if !definition.type_parameters.is_empty() {
                declarations.insert(definition.name.clone(), definition.type_parameters.len());
            }
            for (_, property_type) in &definition.properties {
                collect_generic_reference(property_type, &mut declarations);
            }
            for method in definition.methods.iter().chain(definition.static_methods.iter()) {
                collect_generic_reference(&method.callable.return_type, &mut declarations);
                for parameter in &method.callable.positional_parameters {
                    collect_generic_reference(&parameter.r#type, &mut declarations);
                }
            }
        }
        for function in &module.functions {
            collect_generic_reference(&function.callable.return_type, &mut declarations);
            for parameter in &function.callable.positional_parameters {
                collect_generic_reference(&parameter.r#type, &mut declarations);
            }
        }
    }
    declarations
        .into_iter()
        .filter(|(_, parameter_count)| *parameter_count > 0)
        .map(|(name, parameter_count)| {
            format!(
                "template <{}> struct {};",
                (0..parameter_count)
                    .map(|index| format!("typename T{}", index))
                    .collect::<Vec<_>>()
                    .join(", "),
                name
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_generic_reference(r#type: &Type, declarations: &mut BTreeMap<String, usize>) {
    match r#type {
        Type::Composite(path) => {
            if !path.type_arguments.is_empty() {
                declarations
                    .entry(path.name.clone())
                    .or_insert(path.type_arguments.len());
                for argument in &path.type_arguments {
                    collect_generic_reference(argument, declarations);
                }
            }
        }
        Type::Pointer(inner) | Type::Array(inner) => collect_generic_reference(inner, declarations),
        Type::Tuple(elements) => {
            for element in elements {
                collect_generic_reference(element, declarations);
            }
        }
        Type::Primitive(_) | Type::Dynamic => {}
    }
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

fn render_function_stub(engine: &TemplateEngine, function: &FunctionDefinition) -> String {
    let signature = format!(
        "{}{} {}({})",
        render_type_parameters(&function.callable.type_parameters),
        render_type_name(&function.callable.return_type),
        function.name,
        render_parameters(&function.callable.positional_parameters)
    );
    let body = render_bridge_call_body(
        &function.name,
        &function.callable.positional_parameters,
        "C",
        None,
        Some(&function.callable.return_type),
        false,
    );

    engine.render(
        CALLER_FUNCTION_STUB,
        &crate::map! {
            "SIGNATURE" => signature.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_type_stub(engine: &TemplateEngine, definition: &TypeDefinition) -> String {
    let mut members = Vec::new();
    members.push("std::string uuid;".to_string());
    members.push(format!("struct ReferenceTag {{}};\n  {}(ReferenceTag) {{}}", definition.name));

    if let Some(constructor) = &definition.default_constructor {
        let signature = format!("{}({})", definition.name, render_parameters(&constructor.positional_parameters));
        let body = render_bridge_call_body(
            &definition.name,
            &constructor.positional_parameters,
            "C",
            None,
            None,
            true,
        );
        members.push(engine.render(
            CALLER_CONSTRUCTOR_STUB,
            &crate::map! {
                "SIGNATURE" => signature.as_str(),
                "BODY" => body.as_str(),
            },
            false,
        ));
    } else {
        let signature = format!("{}()", definition.name);
        let body = render_bridge_call_body(&definition.name, &[], "C", None, None, true);
        members.push(engine.render(
            CALLER_CONSTRUCTOR_STUB,
            &crate::map! {
                "SIGNATURE" => signature.as_str(),
                "BODY" => body.as_str(),
            },
            false,
        ));
    }

    for constructor in &definition.named_constructors {
        let signature = format!(
            "static {} {}({})",
            render_type_name(&constructor.callable.return_type),
            constructor.name,
            render_parameters(&constructor.callable.positional_parameters)
        );
        members.push(format!("{signature} {{ return {}; }}", constructor.name));
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
        "static {} __fromReference(const std::string& value) {{\n    {} instance(ReferenceTag{{}});\n    instance.uuid = value;\n    return instance;\n}}",
        definition.name,
        definition.name
    ));

    let type_parameters = render_type_parameters(&definition.type_parameters);
    let members_block = members.join("\n  ");
    engine.render(
        CALLER_TYPE_STUB,
        &crate::map! {
            "TYPE_PARAMETERS" => type_parameters.as_str(),
            "NAME" => definition.name.as_str(),
            "MEMBERS" => members_block.as_str(),
        },
        false,
    )
}

fn render_method_stub(engine: &TemplateEngine, method: &Method) -> String {
    let mut signature = String::new();
    if method.r#static {
        signature.push_str("static ");
    }
    signature.push_str(&render_type_name(&method.callable.return_type));
    signature.push(' ');
    signature.push_str(&method.name);
    signature.push('(');
    signature.push_str(&render_parameters(&method.callable.positional_parameters));
    signature.push(')');
    if !method.r#static {
        signature.push_str(" const");
    }

    let body = render_bridge_call_body(
        &method.name,
        &method.callable.positional_parameters,
        "M",
        if method.r#static { None } else { Some("this->uuid") },
        Some(&method.callable.return_type),
        false,
    );

    engine.render(
        CALLER_METHOD_STUB,
        &crate::map! {
            "SIGNATURE" => signature.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_bridge_call_body(
    target_name: &str,
    parameters: &[ValueParameter],
    message_kind: &str,
    receiver: Option<&str>,
    return_type: Option<&Type>,
    is_constructor: bool,
) -> String {
    let engine = TemplateEngine::new("{{PLACEHOLDER}}").expect("failed to compile template placeholder");

    let bindings = parameters
        .iter()
        .map(|parameter| {
            let local_name = parameter_name_binding(&parameter.name);
            format!(
                "const ptip_ffi::Parameter {local_name} = {};",
                render_parameter_value_cpp(&parameter.r#type, &parameter.name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n    ");

    let receiver_prefix = receiver.map_or_else(
        || {
            format!(
                "C\\n\" + ptip_ffi::serialize_invocation_path(\"\", \"{target_name}\") + \"\\n\" + return_sink",
                target_name = target_name,
            )
        },
        |receiver_name| {
            format!(
                "{message_kind}\\n\" + {receiver_name} + \"\\n\" + ptip_ffi::encode_base64_no_pad_utf8(\"{target_name}\") + \"\\n\" + return_sink",
                message_kind = message_kind,
                receiver_name = receiver_name,
                target_name = target_name,
            )
        },
    );
    let send_message = format!(
        "bridge.send_message(\"{}{});",
        receiver_prefix,
        parameters
            .iter()
            .map(|parameter| {
                let local_name = parameter_name_binding(&parameter.name);
                format!(" + \"\\n\" + ptip_ffi::encode_parameter_line({local_name})")
            })
            .collect::<Vec<_>>()
            .join("")
    );

    let error_handling = if is_constructor {
        CALLER_ERROR_HANDLING_CONSTRUCTOR.to_string()
    } else {
        CALLER_ERROR_HANDLING_FUNCTION.to_string()
    };

    let return_statement = if is_constructor {
        String::new()
    } else {
        let decoded_value_line = "const auto decoded_value = ptip_ffi::decode_parameter_line(lines[2]);";
        let return_stmt = render_bridge_return_statement(
            return_type.expect("non-constructor call should provide a return type"),
            "decoded_value",
        );
        format!("{decoded_value_line}\n{return_stmt}")
    };

    engine.render(
        CALLER_BRIDGE_CALL_BODY,
        &crate::map! {
            "PARAMETER_BINDINGS" => bindings.as_str(),
            "SEND_MESSAGE" => send_message.as_str(),
            "ERROR_HANDLING" => error_handling.as_str(),
            "RETURN_STATEMENT" => return_statement.as_str(),
        },
        true,
    )
}

fn parameter_name_binding(name: &str) -> String {
    // Always use a suffix to avoid shadowing the original parameter name
    format!("{name}_encoded")
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
        Type::Composite(_) => format!("ptip_ffi::encode_value({})", name),
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
        Type::Array(_) | Type::Tuple(_) => {
            "ptip_ffi::Parameter{ptip_ffi::ParameterKind::String, std::string(\"\")}".to_string()
        }
        Type::Dynamic => "ptip_ffi::encode_any_value({})".replace("{}", name),
    }
}

fn render_bridge_return_statement(r#type: &Type, value_name: &str) -> String {
    match r#type {
        Type::Primitive(crate::features::PrimitiveType::Number) => format!(
            "    if ({value_name}.kind == ptip_ffi::ParameterKind::Float || {value_name}.kind == ptip_ffi::ParameterKind::Integer) {{\n        return std::stod({value_name}.value);\n    }}\n    throw std::runtime_error(\"Unexpected return type\");"
        ),
        Type::Primitive(crate::features::PrimitiveType::String) => format!(
            "    if ({value_name}.kind == ptip_ffi::ParameterKind::String) {{\n        return {value_name}.value;\n    }}\n    throw std::runtime_error(\"Unexpected return type\");"
        ),
        Type::Primitive(crate::features::PrimitiveType::Boolean) => format!(
            "    if ({value_name}.kind == ptip_ffi::ParameterKind::Boolean) {{\n        return {value_name}.value == \"1\";\n    }}\n    throw std::runtime_error(\"Unexpected return type\");"
        ),
        Type::Dynamic => format!(
            "    return ptip_ffi::decode_any_value({value_name});",
            value_name = value_name
        ),
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
                format!("    return ptip_ffi::decode_value<{type_name}>({value_name});", value_name = value_name, type_name = type_name)
            } else {
                format!(
                    "    if ({value_name}.kind == ptip_ffi::ParameterKind::Reference) {{\n        return ptip_ffi::decode_value<{type_name}>({value_name});\n    }}\n    throw std::runtime_error(\"Unexpected return type\");",
                    value_name = value_name,
                    type_name = type_name,
                )
            }
        }
        Type::Pointer(inner) => render_bridge_return_statement(inner, value_name),
        Type::Array(_) | Type::Tuple(_) => "    throw std::runtime_error(\"Unsupported return type\");".to_string(),
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
        Type::Dynamic => "std::any".to_string(),
        Type::Composite(path) => {
            if path.name == "Map" && path.type_arguments.len() == 2 {
                return format!(
                    "std::map<{}, {}>",
                    render_type_name(&path.type_arguments[0]),
                    render_type_name(&path.type_arguments[1])
                );
            }
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
            .map(|parameter| {
                let default_type = match parameter.name.as_str() {
                    "K" => "std::string",
                    _ => "long long",
                };
                format!("typename {} = {}", parameter.name, default_type)
            })
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
