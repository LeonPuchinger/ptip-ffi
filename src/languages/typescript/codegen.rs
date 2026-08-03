use std::{collections::HashMap, path::PathBuf};

use crate::{
    codegen::CodegenOutput,
    codegen::template::TemplateEngine,
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

pub fn generate_caller(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let stubs = render_caller_stubs(&modules);
    let engine = TemplateEngine::new("/* {{NAME}} */").expect("failed to compile caller template");
    let rendered_index = engine.render(
        CALLER_INDEX,
        &HashMap::from([("STUBS", stubs.as_str())]),
        true,
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

pub fn generate_callee(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    // dummy implementation
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

fn render_caller_stubs(modules: &[&Module]) -> String {
    let mut stubs = Vec::new();
    for module in modules {
        let module_path = module.path.format(".");
        for function in &module.functions {
            stubs.push(render_function_stub(&module_path, function));
        }
        for definition in &module.types {
            stubs.push(render_type_stub(&module_path, definition));
        }
    }
    stubs.join("\n\n")
}

fn render_function_stub(module_path: &str, function: &FunctionDefinition) -> String {
    render_call_like_stub(
        &format!("export function {}", function.name),
        &function.callable,
        &render_call_body(
            module_path,
            &function.name,
            &function.callable,
            &function.callable.return_type,
            false,
            None,
        ),
        &render_type_parameters(&function.callable.type_parameters),
        &render_type_annotation(&function.callable.return_type),
    )
}

fn render_type_stub(module_path: &str, definition: &TypeDefinition) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!("export class {} {{\n", definition.name));
    rendered.push_str("    readonly uuid: string;\n");
    if let Some(constructor) = &definition.default_constructor {
        rendered.push('\n');
        rendered.push_str(&render_constructor_stub(
            module_path,
            &definition.name,
            constructor,
        ));
    }
    rendered.push('\n');
    rendered.push_str(&render_reference_factory(&definition.name));
    for property in &definition.properties {
        rendered.push('\n');
        rendered.push_str(&render_property_getter(&definition.name, property));
        rendered.push('\n');
        rendered.push_str(&render_property_setter(&property.0, &property.1));
    }
    for method in &definition.methods {
        rendered.push('\n');
        rendered.push_str(&render_method_stub(&definition.name, method));
    }
    for method in &definition.static_methods {
        rendered.push('\n');
        rendered.push_str(&render_static_method_stub(
            module_path,
            &definition.name,
            method,
        ));
    }
    for constructor in &definition.named_constructors {
        rendered.push('\n');
        rendered.push_str(&render_static_named_constructor_stub(
            module_path,
            &definition.name,
            constructor,
        ));
    }
    rendered.push_str("\n}\n");
    rendered
}

fn render_constructor_stub(
    module_path: &str,
    type_name: &str,
    constructor: &AnonymousCallable,
) -> String {
    let parameters = render_parameters(&constructor.positional_parameters);
    let values = render_parameters_values(&constructor.positional_parameters);
    let body = format!(
        "        this.uuid = crypto.randomUUID();\n        const bridge = establishBridge();\n        bridge.send(\n            new CallMessage({{\n                modulePath: \"{module_path}\",\n                callee: {{ kind: \"function\", name: \"{type_name}\" }},\n                returnSink: this.uuid,\n                positional: [{values}],\n                named: new Map(),\n            }}),\n        );\n        const response = bridge.nextMessage();\n        if (response === null) {{\n            throw new Error(\"No response received from the bridge\");\n        }}\n        if (response.kind === \"error\") {{\n            throw (response as ErrorMessage).error.value;\n        }}\n        if (response.kind === \"send\") {{\n            const sendMessage = response as SendMessage;\n            if (sendMessage.reference !== this.uuid) {{\n                throw new Error(\"Mismatched UUID in response\");\n            }}\n            if (sendMessage.value.kind !== \"reference\" || sendMessage.value.value !== this.uuid) {{\n                throw new Error(\"Unexpected message kind or value\");\n            }}\n        }} else {{\n            throw new Error(`Unexpected message kind: ${{response.kind}}`);\n        }}\n        finalizationRegistry.register(this, this.uuid);\n"
    );
    format!(
        "    constructor({parameters}) {{\n{body}    }}\n",
        parameters = parameters,
        body = body
    )
}

fn render_reference_factory(type_name: &str) -> String {
    format!(
        "    static __fromReference(uuid: string): {type_name} {{\n        const reference = Object.create({type_name}.prototype) as {type_name} & {{ uuid: string }};\n        reference.uuid = uuid;\n        finalizationRegistry.register(reference, uuid);\n        return reference;\n    }}\n"
    )
}

fn render_property_getter(type_name: &str, property: &(String, Type)) -> String {
    let property_type = render_type_annotation(&property.1);
    let response = render_response_body(&property.1, type_name, "sendMessage.value", false);
    format!(
        "    get {name}(): {property_type} {{\n        const returnSink = crypto.randomUUID();\n        const bridge = establishBridge();\n        bridge.send(\n            new RequestMessage({{\n                parent: this.uuid,\n                accessor: \"{name}\",\n                valueSink: returnSink,\n            }}),\n        );\n        const response = bridge.nextMessage();\n        if (response === null) {{\n            throw new Error(\"No response received from the bridge\");\n        }}\n        if (response.kind === \"error\") {{\n            throw (response as ErrorMessage).error.value;\n        }}\n        if (response.kind === \"send\") {{\n            const sendMessage = response as SendMessage;\n            if (sendMessage.reference !== returnSink) {{\n                throw new Error(\"Mismatched UUID in response\");\n            }}\n{response}\n        }}\n        throw new Error(`Unexpected message kind: ${{response.kind}}`);\n    }}",
        name = property.0,
        property_type = property_type,
        response = indent_block(&response, 3)
    )
}

fn render_property_setter(name: &str, property_type: &Type) -> String {
    let value_type = render_type_annotation(property_type);
    let value = render_parameter_value_expression(property_type, "value");
    format!(
        "    set {name}(value: {value_type}) {{\n        const acknowledgeSink = crypto.randomUUID();\n        const bridge = establishBridge();\n        bridge.send(\n            new UpdateMessage({{\n                parent: this.uuid,\n                accessor: \"{name}\",\n                acknowledgeSink: acknowledgeSink,\n                value: {value},\n            }}),\n        );\n        const response = bridge.nextMessage();\n        if (response === null) {{\n            throw new Error(\"No response received from the bridge\");\n        }}\n        if (response.kind === \"error\") {{\n            throw (response as ErrorMessage).error.value;\n        }}\n        if (response.kind === \"acknowledge\") {{\n            const acknowledgeMessage = response as AcknowledgeMessage;\n            if (acknowledgeMessage.reference !== acknowledgeSink) {{\n                throw new Error(\"Mismatched UUID in response\");\n            }}\n            return;\n        }}\n        throw new Error(`Unexpected message kind: ${{response.kind}}`);\n    }}",
        name = name,
        value_type = value_type,
        value = value
    )
}

fn render_method_stub(type_name: &str, method: &Method) -> String {
    render_call_like_stub(
        &format!("    {}", method.name),
        &method.callable,
        &render_call_body(
            "",
            &method.name,
            &method.callable,
            &method.callable.return_type,
            false,
            Some(type_name),
        ),
        &render_type_parameters(&method.callable.type_parameters),
        &render_type_annotation(&method.callable.return_type),
    )
    .replacen("export function", "", 1)
}

fn render_static_method_stub(module_path: &str, type_name: &str, method: &Method) -> String {
    let signature_name = format!("static {}", method.name);
    let body = render_call_body(
        module_path,
        &format!("{}.{}", type_name, method.name),
        &method.callable,
        &method.callable.return_type,
        false,
        Some(type_name),
    );
    format!(
        "    {signature_name}{type_parameters}({parameters}): {return_type} {{\n{body}    }}\n",
        signature_name = signature_name,
        type_parameters = render_type_parameters(&method.callable.type_parameters),
        parameters = render_parameters(&method.callable.positional_parameters),
        return_type = render_type_annotation(&method.callable.return_type),
        body = body,
    )
}

fn render_static_named_constructor_stub(
    module_path: &str,
    type_name: &str,
    constructor: &FunctionDefinition,
) -> String {
    let body = render_call_body(
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
    format!(
        "    static {name}{type_parameters}({parameters}): {return_type} {{\n{body}    }}\n",
        name = constructor.name,
        type_parameters = render_type_parameters(&constructor.callable.type_parameters),
        parameters = render_parameters(&constructor.callable.positional_parameters),
        return_type = render_type_annotation(&Type::Composite(crate::features::TypePath::new(
            crate::features::ModulePath::empty(),
            type_name.to_string(),
        ))),
        body = body,
    )
}

fn render_call_like_stub(
    signature_prefix: &str,
    callable: &AnonymousCallable,
    body: &str,
    type_parameters: &str,
    return_type: &str,
) -> String {
    format!(
        "{signature_prefix}{type_parameters}({parameters}): {return_type} {{\n{body}    }}\n",
        signature_prefix = signature_prefix,
        type_parameters = type_parameters,
        parameters = render_parameters(&callable.positional_parameters),
        return_type = return_type,
        body = body,
    )
}

fn render_call_body(
    module_path: &str,
    callee_name: &str,
    callable: &AnonymousCallable,
    return_type: &Type,
    is_method: bool,
    receiver_type_name: Option<&str>,
) -> String {
    let positional_values = render_parameters_values(&callable.positional_parameters);
    let mut body = String::new();
    if is_method {
        body.push_str("        const returnSink = crypto.randomUUID();\n        const bridge = establishBridge();\n        bridge.send(\n            new MethodMessage({\n                calledReference: this.uuid,\n                methodName: \"");
        body.push_str(callee_name);
        body.push_str(
            "\",\n                returnSink: returnSink,\n                positional: [",
        );
        body.push_str(&positional_values);
        body.push_str("],\n                named: new Map(),\n            }),\n        );\n");
    } else {
        body.push_str("        const returnSink = crypto.randomUUID();\n        const bridge = establishBridge();\n        bridge.send(\n            new CallMessage({\n                modulePath: \"");
        body.push_str(module_path);
        body.push_str("\",\n                callee: { kind: \"function\", name: \"");
        body.push_str(callee_name);
        body.push_str(
            "\" },\n                returnSink: returnSink,\n                positional: [",
        );
        body.push_str(&positional_values);
        body.push_str("],\n                named: new Map(),\n            }),\n        );\n");
    }
    body.push_str("        const response = bridge.nextMessage();\n        if (response === null) {\n            throw new Error(\"No response received from the bridge\");\n        }\n        if (response.kind === \"error\") {\n            throw (response as ErrorMessage).error.value;\n        }\n        if (response.kind === \"send\") {\n            const sendMessage = response as SendMessage;\n            if (sendMessage.reference !== returnSink) {\n                throw new Error(\"Mismatched UUID in response\");\n            }\n");
    body.push_str(&render_response_body(
        return_type,
        receiver_type_name.unwrap_or(callee_name),
        "sendMessage.value",
        false,
    ));
    body.push_str(
        "        }\n        throw new Error(`Unexpected message kind: ${response.kind}`);\n",
    );
    body
}

fn render_response_body(
    return_type: &Type,
    composite_type_name: &str,
    send_message_value: &str,
    _indent: bool,
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

fn indent_block(content: &str, indentation_level: usize) -> String {
    let indentation = "    ".repeat(indentation_level);
    content
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indentation}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
