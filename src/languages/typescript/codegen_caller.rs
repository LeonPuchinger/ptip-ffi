use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{
        AnonymousCallable, FunctionDefinition, Method, Module, PrimitiveType, Type, TypeDefinition,
        TypeParameter, ValueParameter,
    },
};

// Common assets (shared between caller and callee)
const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");
// Caller assets
const CALLER_PACKAGE_JSON: &str = include_str!("./assets/caller/package.json");
const CALLER_PACKAGE_LOCK_JSON: &str = include_str!("./assets/caller/package-lock.json");
const CALLER_MAIN: &str = include_str!("./assets/caller/main.ts");
const CALLER_INDEX: &str = include_str!("./assets/caller/index.ts");
const CALLER_FUNCTION_STUB: &str = include_str!("./assets/caller/function_stub.ts");
const CALLER_CLASS_STUB: &str = include_str!("./assets/caller/class_stub.ts");
const CALLER_CONSTRUCTOR_STUB: &str = include_str!("./assets/caller/constructor_stub.ts");
const CALLER_REFERENCE_FACTORY: &str = include_str!("./assets/caller/reference_factory.ts");
const CALLER_PROPERTY_GETTER: &str = include_str!("./assets/caller/property_getter.ts");
const CALLER_PROPERTY_SETTER: &str = include_str!("./assets/caller/property_setter.ts");
const CALLER_METHOD_STUB: &str = include_str!("./assets/caller/method_stub.ts");
const CALLER_STATIC_METHOD_STUB: &str = include_str!("./assets/caller/static_method_stub.ts");
const CALLER_STATIC_NAMED_CONSTRUCTOR_STUB: &str =
    include_str!("./assets/caller/static_named_constructor_stub.ts");
const CALLER_CALL_FUNCTION_BODY: &str = include_str!("./assets/caller/call_function_body.ts");
const CALLER_CALL_METHOD_BODY: &str = include_str!("./assets/caller/call_method_body.ts");

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine =
        TemplateEngine::new("/* {{NAME}} */").expect("failed to compile template placeholder");

    let stubs = render_caller_stubs(&engine, &modules);
    let rendered_index = engine.render(
        CALLER_INDEX,
        &crate::map! {
            "STUBS" => stubs.as_str(),
        },
        false,
    );
    vec![
        CodegenOutput {
            path: PathBuf::from("ffi/socket.ts"),
            content: SOCKET_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/bridge.ts"),
            content: BRIDGE_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/main.ts"),
            content: CALLER_MAIN.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("package.json"),
            content: CALLER_PACKAGE_JSON.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("package-lock.json"),
            content: CALLER_PACKAGE_LOCK_JSON.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("index.ts"),
            content: rendered_index,
        },
    ]
}

fn render_caller_stubs(engine: &TemplateEngine, modules: &[&Module]) -> String {
    let mut stubs = Vec::new();
    for module in modules {
        let module_path = module.path.format(".");
        for function in &module.functions {
            stubs.push(render_function_stub(engine, &module_path, function));
        }
        for definition in &module.types {
            stubs.push(render_type_stub(engine, &module_path, definition));
        }
    }
    stubs.join("\n\n")
}

fn render_function_stub(
    engine: &TemplateEngine,
    module_path: &str,
    function: &FunctionDefinition,
) -> String {
    let type_parameters = render_type_parameters(&function.callable.type_parameters);
    let parameters = render_parameters(&function.callable.positional_parameters);
    let return_type = render_type_annotation(&function.callable.return_type);
    let body = render_call_body(
        engine,
        module_path,
        &function.name,
        &function.callable,
        &function.callable.return_type,
        false,
        None,
    );
    engine.render(
        CALLER_FUNCTION_STUB,
        &crate::map! {
            "NAME" => function.name.as_str(),
            "TYPE_PARAMETERS" => type_parameters.as_str(),
            "PARAMETERS" => parameters.as_str(),
            "RETURN_TYPE" => return_type.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_type_stub(
    engine: &TemplateEngine,
    module_path: &str,
    definition: &TypeDefinition,
) -> String {
    let mut members = Vec::new();
    if let Some(constructor) = &definition.default_constructor {
        members.push(render_constructor_stub(
            engine,
            module_path,
            &definition.name,
            constructor,
        ));
    }
    members.push(render_reference_factory(engine, &definition.name));
    for property in &definition.properties {
        members.push(render_property_getter(engine, &definition.name, property));
        members.push(render_property_setter(engine, &property.0, &property.1));
    }
    for method in &definition.methods {
        members.push(render_method_stub(engine, &definition.name, method));
    }
    for method in &definition.static_methods {
        members.push(render_static_method_stub(
            engine,
            module_path,
            &definition.name,
            method,
        ));
    }
    for constructor in &definition.named_constructors {
        members.push(render_static_named_constructor_stub(
            engine,
            module_path,
            &definition.name,
            constructor,
        ));
    }
    let members = members.join("\n");
    engine.render(
        CALLER_CLASS_STUB,
        &crate::map! {
            "NAME" => definition.name.as_str(),
            "MEMBERS" => members.as_str(),
        },
        false,
    )
}

fn render_constructor_stub(
    engine: &TemplateEngine,
    module_path: &str,
    type_name: &str,
    constructor: &AnonymousCallable,
) -> String {
    let parameters = render_parameters(&constructor.positional_parameters);
    let positional_values = render_parameters_values(&constructor.positional_parameters);
    engine.render(
        CALLER_CONSTRUCTOR_STUB,
        &crate::map! {
            "PARAMETERS" => parameters.as_str(),
            "MODULE_PATH" => module_path,
            "TYPE_NAME" => type_name,
            "POSITIONAL_VALUES" => positional_values.as_str(),
        },
        false,
    )
}

fn render_reference_factory(engine: &TemplateEngine, type_name: &str) -> String {
    engine.render(
        CALLER_REFERENCE_FACTORY,
        &crate::map! {
            "NAME" => type_name,
        },
        false,
    )
}

fn render_property_getter(
    engine: &TemplateEngine,
    type_name: &str,
    property: &(String, Type),
) -> String {
    let return_type = render_type_annotation(&property.1);
    let body = render_response_body(&property.1, type_name, "sendMessage.value");
    engine.render(
        CALLER_PROPERTY_GETTER,
        &crate::map! {
            "NAME" => property.0.as_str(),
            "RETURN_TYPE" => return_type.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_property_setter(engine: &TemplateEngine, name: &str, property_type: &Type) -> String {
    let value_type = render_type_annotation(property_type);
    let value = render_parameter_value_expression(property_type, "value");
    engine.render(
        CALLER_PROPERTY_SETTER,
        &crate::map! {
            "NAME" => name,
            "VALUE_TYPE" => value_type.as_str(),
            "VALUE" => value.as_str(),
        },
        false,
    )
}

fn render_method_stub(engine: &TemplateEngine, type_name: &str, method: &Method) -> String {
    let type_parameters = render_type_parameters(&method.callable.type_parameters);
    let parameters = render_parameters(&method.callable.positional_parameters);
    let return_type = render_type_annotation(&method.callable.return_type);
    let body = render_call_body(
        engine,
        "",
        &method.name,
        &method.callable,
        &method.callable.return_type,
        false,
        Some(type_name),
    );
    engine.render(
        CALLER_METHOD_STUB,
        &crate::map! {
            "NAME" => method.name.as_str(),
            "TYPE_PARAMETERS" => type_parameters.as_str(),
            "PARAMETERS" => parameters.as_str(),
            "RETURN_TYPE" => return_type.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_static_method_stub(
    engine: &TemplateEngine,
    module_path: &str,
    type_name: &str,
    method: &Method,
) -> String {
    let type_parameters = render_type_parameters(&method.callable.type_parameters);
    let parameters = render_parameters(&method.callable.positional_parameters);
    let return_type = render_type_annotation(&method.callable.return_type);
    let body = render_call_body(
        engine,
        module_path,
        &format!("{}.{}", type_name, method.name),
        &method.callable,
        &method.callable.return_type,
        false,
        Some(type_name),
    );
    engine.render(
        CALLER_STATIC_METHOD_STUB,
        &crate::map! {
            "NAME" => method.name.as_str(),
            "TYPE_PARAMETERS" => type_parameters.as_str(),
            "PARAMETERS" => parameters.as_str(),
            "RETURN_TYPE" => return_type.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_static_named_constructor_stub(
    engine: &TemplateEngine,
    module_path: &str,
    type_name: &str,
    constructor: &FunctionDefinition,
) -> String {
    let return_type = render_type_annotation(&Type::Composite(crate::features::TypePath::new(
        crate::features::ModulePath::empty(),
        type_name.to_string(),
    )));
    let type_parameters = render_type_parameters(&constructor.callable.type_parameters);
    let parameters = render_parameters(&constructor.callable.positional_parameters);
    let body = render_call_body(
        engine,
        module_path,
        &format!("{}.{}", type_name, constructor.name),
        &constructor.callable,
        &Type::Composite(crate::features::TypePath::new(
            crate::features::ModulePath::empty(),
            type_name.to_string(),
        )),
        false,
        Some(type_name),
    );
    engine.render(
        CALLER_STATIC_NAMED_CONSTRUCTOR_STUB,
        &crate::map! {
            "NAME" => constructor.name.as_str(),
            "TYPE_PARAMETERS" => type_parameters.as_str(),
            "PARAMETERS" => parameters.as_str(),
            "RETURN_TYPE" => return_type.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_call_body(
    engine: &TemplateEngine,
    module_path: &str,
    callee_name: &str,
    callable: &AnonymousCallable,
    return_type: &Type,
    is_method: bool,
    receiver_type_name: Option<&str>,
) -> String {
    let positional_values = render_parameters_values(&callable.positional_parameters);
    let response_body = render_response_body(
        return_type,
        receiver_type_name.unwrap_or(callee_name),
        "sendMessage.value",
    );
    if is_method {
        return engine.render(
            CALLER_CALL_METHOD_BODY,
            &crate::map! {
                "METHOD_NAME" => callee_name,
                "POSITIONAL_VALUES" => positional_values.as_str(),
                "RESPONSE_BODY" => response_body.as_str(),
            },
            false,
        );
    }
    engine.render(
        CALLER_CALL_FUNCTION_BODY,
        &crate::map! {
            "MODULE_PATH" => module_path,
            "CALLEE_NAME" => callee_name,
            "POSITIONAL_VALUES" => positional_values.as_str(),
            "RESPONSE_BODY" => response_body.as_str(),
        },
        false,
    )
}

fn render_response_body(
    return_type: &Type,
    composite_type_name: &str,
    send_message_value: &str,
) -> String {
    match return_type {
        Type::Primitive(PrimitiveType::Number) => format!(
            "            if ({send_message_value}.kind === \"integer\" || {send_message_value}.kind === \"float\") {{\n                return {send_message_value}.value;\n            }}\n            throw new Error(\"Unexpected message kind\");\n",
        ),
        Type::Primitive(PrimitiveType::String) => format!(
            "            if ({send_message_value}.kind === \"string\") {{\n                return {send_message_value}.value;\n            }}\n            throw new Error(\"Unexpected message kind\");\n",
        ),
        Type::Primitive(PrimitiveType::Boolean) => format!(
            "            if ({send_message_value}.kind === \"boolean\") {{\n                return {send_message_value}.value;\n            }}\n            throw new Error(\"Unexpected message kind\");\n",
        ),
        Type::Composite(_) => format!(
            "            if ({send_message_value}.kind !== \"reference\") {{\n                throw new Error(\"Unexpected message kind\");\n            }}\n            return {composite_type_name}.__fromReference({send_message_value}.value);\n",
            composite_type_name = composite_type_name,
            send_message_value = send_message_value,
        ),
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => {
            format!("            return {send_message_value}.value as never;\n",)
        }
    }
}

fn render_type_annotation(r#type: &Type) -> String {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => "number".to_string(),
        Type::Primitive(PrimitiveType::String) => "string".to_string(),
        Type::Primitive(PrimitiveType::Boolean) => "boolean".to_string(),
        Type::Composite(path) => path.name.clone(),
        Type::Array(inner) => format!("{}[]", render_type_annotation(inner)),
        Type::Tuple(elements) => format!(
            "[{}]",
            elements
                .iter()
                .map(render_type_annotation)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Dynamic => "any".to_string(),
    }
}

fn render_parameter_value_expression(r#type: &Type, name: &str) -> String {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => format!(
            "{{\n                    kind: Number.isInteger({name}) ? \"integer\" : \"float\",\n                    value: {name},\n                }}"
        ),
        Type::Primitive(PrimitiveType::String) => format!(
            "{{\n                    kind: \"string\",\n                    value: {name},\n                }}"
        ),
        Type::Primitive(PrimitiveType::Boolean) => format!(
            "{{\n                    kind: \"boolean\",\n                    value: {name},\n                }}"
        ),
        Type::Composite(_) => format!(
            "{{\n                    kind: \"reference\",\n                    value: {name}.uuid,\n                }}"
        ),
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => format!(
            "{{\n                    kind: \"string\",\n                    value: JSON.stringify({name}),\n                }}"
        ),
    }
}

fn render_parameters(parameters: &[ValueParameter]) -> String {
    parameters
        .iter()
        .map(render_parameter_signature)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_parameter_signature(parameter: &ValueParameter) -> String {
    let type_annotation = if parameter.variadic {
        format!("{}[]", render_type_annotation(&parameter.r#type))
    } else {
        render_type_annotation(&parameter.r#type)
    };
    let optional = if parameter.required || parameter.variadic {
        ""
    } else {
        "?"
    };
    let rest = if parameter.variadic { "..." } else { "" };
    format!(
        "{rest}{name}{optional}: {type_annotation}",
        rest = rest,
        name = parameter.name,
        optional = optional,
        type_annotation = type_annotation
    )
}

fn render_parameters_values(parameters: &[ValueParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| render_parameter_value_expression(&parameter.r#type, &parameter.name))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_type_parameters(parameters: &[TypeParameter]) -> String {
    if parameters.is_empty() {
        return String::new();
    }
    let rendered = parameters
        .iter()
        .map(|parameter| match &parameter.default {
            Some(default) => format!("{} = {}", parameter.name, render_type_annotation(default)),
            None => parameter.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{rendered}>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number_parameter(name: &str) -> ValueParameter {
        ValueParameter {
            name: name.to_string(),
            r#type: Type::Primitive(PrimitiveType::Number),
            required: true,
            variadic: false,
            nullable: false,
        }
    }

    #[test]
    fn generate_caller_renders_stubs_into_index_template() {
        let module = Module {
            path: crate::features::ModulePath::empty(),
            functions: vec![FunctionDefinition {
                name: "trim_whitespace".to_string(),
                callable: AnonymousCallable {
                    positional_parameters: vec![ValueParameter {
                        name: "str".to_string(),
                        r#type: Type::Primitive(PrimitiveType::String),
                        required: true,
                        variadic: false,
                        nullable: false,
                    }],
                    named_parameters: Vec::new(),
                    return_type: Type::Primitive(PrimitiveType::String),
                    type_parameters: Vec::new(),
                },
            }],
            types: vec![TypeDefinition {
                name: "Point".to_string(),
                properties: vec![
                    ("x".to_string(), Type::Primitive(PrimitiveType::Number)),
                    ("y".to_string(), Type::Primitive(PrimitiveType::Number)),
                ],
                default_constructor: Some(AnonymousCallable {
                    positional_parameters: vec![number_parameter("x"), number_parameter("y")],
                    named_parameters: Vec::new(),
                    return_type: Type::Composite(crate::features::TypePath::new(
                        crate::features::ModulePath::empty(),
                        "Point".to_string(),
                    )),
                    type_parameters: Vec::new(),
                }),
                named_constructors: Vec::new(),
                methods: vec![Method {
                    name: "distance_to_origin".to_string(),
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
        let index_ts = outputs
            .iter()
            .find(|output| output.path == PathBuf::from("index.ts"))
            .expect("index.ts should be generated");

        assert!(
            index_ts
                .content
                .contains("export function trim_whitespace(str: string): string")
        );
        assert!(index_ts.content.contains("export class Point"));
        assert!(index_ts.content.contains("distance_to_origin(): number"));
        assert!(!index_ts.content.contains("{{STUBS}}"));
    }
}
