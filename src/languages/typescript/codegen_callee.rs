use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{
        AnonymousCallable, FunctionDefinition, Method, Module, ModulePath, PrimitiveType, Type,
        TypeDefinition, ValueParameter,
    },
};

const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");

const CALLEE_PACKAGE_JSON: &str = include_str!("./assets/callee/package.json");
const CALLEE_PACKAGE_LOCK_JSON: &str = include_str!("./assets/callee/package-lock.json");
const CALLEE_MAIN: &str = include_str!("./assets/callee/main.ts");
const CALLEE_DISPATCH: &str = include_str!("./assets/callee/dispatch.ts");
const CALLEE_FUNCTION_CASE: &str = include_str!("./assets/callee/function_case.ts");
const CALLEE_CONSTRUCTOR_CASE: &str = include_str!("./assets/callee/constructor_case.ts");
const CALLEE_STATIC_METHOD_CASE: &str = include_str!("./assets/callee/static_method_case.ts");
const CALLEE_STATIC_NAMED_CONSTRUCTOR_CASE: &str =
    include_str!("./assets/callee/static_named_constructor_case.ts");
const CALLEE_STATIC_METHOD_MODULE_SWITCH: &str =
    include_str!("./assets/callee/static_method_module_switch.ts");
const CALLEE_METHOD_CASE: &str = include_str!("./assets/callee/method_case.ts");
const CALLEE_REQUEST_CASE: &str = include_str!("./assets/callee/request_case.ts");
const CALLEE_UPDATE_CASE: &str = include_str!("./assets/callee/update_case.ts");
const CALLEE_MODULE_SWITCH: &str = include_str!("./assets/callee/module_switch.ts");
const CALLEE_TYPE_SWITCH: &str = include_str!("./assets/callee/type_switch.ts");
const CALLEE_METHOD_BLOCK: &str = include_str!("./assets/callee/method_block.ts");
const CALLEE_REQUEST_BLOCK: &str = include_str!("./assets/callee/request_block.ts");
const CALLEE_UPDATE_BLOCK: &str = include_str!("./assets/callee/update_block.ts");

pub fn generate_callee(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine =
        TemplateEngine::new("/* {{NAME}} */").expect("failed to compile template placeholder");
    let rendered_dispatch = render_dispatch(&engine, &modules);
    vec![
        CodegenOutput {
            path: PathBuf::from("bridge.ts"),
            content: BRIDGE_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("socket.ts"),
            content: SOCKET_IMPLEMENTATION.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("package.json"),
            content: CALLEE_PACKAGE_JSON.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("package-lock.json"),
            content: CALLEE_PACKAGE_LOCK_JSON.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("main.ts"),
            content: CALLEE_MAIN.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("dispatch.ts"),
            content: rendered_dispatch,
        },
    ]
}

fn render_dispatch(engine: &TemplateEngine, modules: &[&Module]) -> String {
    let imports = render_dispatch_imports(modules);
    let function_cases = render_function_cases(engine, modules);
    let static_method_cases = render_static_method_cases(engine, modules);
    let method_cases = render_method_cases(engine, modules);
    let request_cases = render_request_cases(engine, modules);
    let update_cases = render_update_cases(engine, modules);
    engine.render(
        CALLEE_DISPATCH,
        &crate::map! {
            "IMPORTS" => imports.as_str(),
            "FUNCTION_CASES" => function_cases.as_str(),
            "STATIC_METHOD_CASES" => static_method_cases.as_str(),
            "METHOD_CASES" => method_cases.as_str(),
            "REQUEST_CASES" => request_cases.as_str(),
            "UPDATE_CASES" => update_cases.as_str(),
        },
        false,
    )
}

fn render_dispatch_imports(modules: &[&Module]) -> String {
    modules
        .iter()
        .map(|module| {
            format!(
                "import * as {} from \"{}\";",
                module_namespace_name(&module.path),
                library_import_path(&module.path)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn library_import_path(module_path: &ModulePath) -> String {
    if module_path.segments.is_empty() {
        "./library/index.ts".to_string()
    } else {
        format!("./library/{}/index.ts", module_path.format("/"))
    }
}

fn render_method_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    method: &Method,
    module_path: &ModulePath,
) -> String {
    let arguments = render_call_arguments(&method.callable.positional_parameters, module_path);
    let body = render_return_body(&method.callable.return_type, "result", "returnSink");
    engine.render(
        CALLEE_METHOD_CASE,
        &crate::map! {
            "METHOD_NAME" => method.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "TYPE_NAME" => definition.name.as_str(),
            "ARGUMENTS" => arguments.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_request_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    property: &(String, Type),
) -> String {
    let body = render_request_body(&property.1, &property.0);
    engine.render(
        CALLEE_REQUEST_CASE,
        &crate::map! {
            "ACCESSOR" => property.0.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "TYPE_NAME" => definition.name.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_update_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    property: &(String, Type),
    current_module: &ModulePath,
) -> String {
    let value = render_parameter_value_expression(&property.1, "value", current_module);
    let body = render_update_body(&property.0, &value);
    engine.render(
        CALLEE_UPDATE_CASE,
        &crate::map! {
            "ACCESSOR" => property.0.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "TYPE_NAME" => definition.name.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_type_annotation(r#type: &Type, current_module: &ModulePath) -> String {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => "number".to_string(),
        Type::Primitive(PrimitiveType::String) => "string".to_string(),
        Type::Primitive(PrimitiveType::Boolean) => "boolean".to_string(),
        Type::Composite(path) => format!("{}.{}", module_namespace_name(&path.module_path), path.name),
        Type::Array(inner) => format!("{}[]", render_type_annotation(inner, current_module)),
        Type::Tuple(elements) => format!(
            "[{}]",
            elements
                .iter()
                .map(|element| render_type_annotation(element, current_module))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Dynamic => "any".to_string(),
    }
}

fn render_function_cases(engine: &TemplateEngine, modules: &[&Module]) -> String {
    modules
        .iter()
        .map(|module| {
            let module_path = module.path.format("/");
            let module_namespace = module_namespace_name(&module.path);
            let mut cases = Vec::new();
            for function in &module.functions {
                cases.push(render_function_case(
                    engine,
                    &module_namespace,
                    function,
                    &module.path,
                ));
            }
            for definition in &module.types {
                if let Some(constructor) = &definition.default_constructor {
                    cases.push(render_constructor_case(
                        engine,
                        &module_namespace,
                        definition,
                        constructor,
                        &module.path,
                    ));
                }
            }
            let cases = cases.join("\n");
            engine.render(
                CALLEE_MODULE_SWITCH,
                &crate::map! {
                    "MODULE_PATH" => module_path.as_str(),
                    "CASES" => cases.as_str(),
                },
                false,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_static_method_cases(engine: &TemplateEngine, modules: &[&Module]) -> String {
    modules
        .iter()
        .map(|module| {
            let module_path = module.path.format("/");
            let module_namespace = module_namespace_name(&module.path);
            let mut type_cases = Vec::new();
            for definition in &module.types {
                let mut method_cases = Vec::new();

                for method in &definition.static_methods {
                    method_cases.push(render_static_method_case(
                        engine,
                        &module_namespace,
                        definition,
                        method,
                        &module.path,
                    ));
                }
                for constructor in &definition.named_constructors {
                    method_cases.push(render_named_constructor_case(
                        engine,
                        &module_namespace,
                        definition,
                        constructor,
                        &module.path,
                    ));
                }
                if !method_cases.is_empty() {
                    let method_cases = method_cases.join("\n");
                    type_cases.push(engine.render(
                        CALLEE_TYPE_SWITCH,
                        &crate::map! {
                            "TYPE_NAME" => definition.name.as_str(),
                            "CASES" => method_cases.as_str(),
                        },
                        false,
                    ));
                }
            }
            let type_cases = type_cases.join("\n");
            engine.render(
                CALLEE_STATIC_METHOD_MODULE_SWITCH,
                &crate::map! {
                    "MODULE_PATH" => module_path.as_str(),
                    "TYPE_CASES" => type_cases.as_str(),
                },
                false,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_method_cases(engine: &TemplateEngine, modules: &[&Module]) -> String {
    modules
        .iter()
        .flat_map(|module| {
            let module_namespace = module_namespace_name(&module.path);
            module.types.iter().filter_map(move |definition| {
                if definition.methods.is_empty() {
                    return None;
                }
                let method_blocks = definition
                    .methods
                    .iter()
                    .map(|method| {
                        render_method_case(
                            engine,
                            &module_namespace,
                            definition,
                            method,
                            &module.path,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(engine.render(
                    CALLEE_METHOD_BLOCK,
                    &crate::map! {
                        "MODULE_NAMESPACE" => module_namespace.as_str(),
                        "TYPE_NAME" => definition.name.as_str(),
                        "CASES" => method_blocks.as_str(),
                    },
                    false,
                ))
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_request_cases(engine: &TemplateEngine, modules: &[&Module]) -> String {
    modules
        .iter()
        .flat_map(|module| {
            let module_namespace = module_namespace_name(&module.path);
            module.types.iter().filter_map(move |definition| {
                if definition.properties.is_empty() {
                    return None;
                }
                let property_blocks = definition
                    .properties
                    .iter()
                    .map(|property| {
                        render_request_case(engine, &module_namespace, definition, property)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(engine.render(
                    CALLEE_REQUEST_BLOCK,
                    &crate::map! {
                        "MODULE_NAMESPACE" => module_namespace.as_str(),
                        "TYPE_NAME" => definition.name.as_str(),
                        "CASES" => property_blocks.as_str(),
                    },
                    false,
                ))
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_update_cases(engine: &TemplateEngine, modules: &[&Module]) -> String {
    modules
        .iter()
        .flat_map(|module| {
            let module_namespace = module_namespace_name(&module.path);
            module.types.iter().filter_map(move |definition| {
                if definition.properties.is_empty() {
                    return None;
                }
                let property_blocks = definition
                    .properties
                    .iter()
                    .map(|property| {
                        render_update_case(
                            engine,
                            &module_namespace,
                            definition,
                            property,
                            &module.path,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(engine.render(
                    CALLEE_UPDATE_BLOCK,
                    &crate::map! {
                        "MODULE_NAMESPACE" => module_namespace.as_str(),
                        "TYPE_NAME" => definition.name.as_str(),
                        "CASES" => property_blocks.as_str(),
                    },
                    false,
                ))
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_function_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    function: &FunctionDefinition,
    module_path: &ModulePath,
) -> String {
    let arguments = render_call_arguments(&function.callable.positional_parameters, module_path);
    let body = render_return_body(&function.callable.return_type, "result", "returnSink");
    engine.render(
        CALLEE_FUNCTION_CASE,
        &crate::map! {
            "CALLEE_NAME" => function.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_constructor_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    constructor: &AnonymousCallable,
    module_path: &ModulePath,
) -> String {
    let arguments = render_call_arguments(&constructor.positional_parameters, module_path);
    let body = render_reference_return_body("result", "returnSink");
    engine.render(
        CALLEE_CONSTRUCTOR_CASE,
        &crate::map! {
            "TYPE_NAME" => definition.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_static_method_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    method: &Method,
    module_path: &ModulePath,
) -> String {
    let arguments = render_call_arguments(&method.callable.positional_parameters, module_path);
    let body = render_return_body(&method.callable.return_type, "result", "returnSink");
    engine.render(
        CALLEE_STATIC_METHOD_CASE,
        &crate::map! {
            "METHOD_NAME" => method.name.as_str(),
            "TYPE_NAME" => definition.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_named_constructor_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    constructor: &FunctionDefinition,
    module_path: &ModulePath,
) -> String {
    let arguments = render_call_arguments(&constructor.callable.positional_parameters, module_path);
    let body = render_reference_return_body("result", "returnSink");
    engine.render(
        CALLEE_STATIC_NAMED_CONSTRUCTOR_CASE,
        &crate::map! {
            "METHOD_NAME" => constructor.name.as_str(),
            "TYPE_NAME" => definition.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_call_arguments(parameters: &[ValueParameter], current_module: &ModulePath) -> String {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            format!(
                "positionalParameters[{index}] as {}",
                render_type_annotation(&parameter.r#type, current_module)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_parameter_value_expression(
    r#type: &Type,
    name: &str,
    current_module: &ModulePath,
) -> String {
    match r#type {
        Type::Primitive(PrimitiveType::Number)
        | Type::Primitive(PrimitiveType::String)
        | Type::Primitive(PrimitiveType::Boolean)
        | Type::Composite(_) => {
            format!("{name} as {}", render_type_annotation(r#type, current_module))
        }
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => {
            format!("JSON.parse({name} as string) as {}", render_type_annotation(r#type, current_module))
        }
    }
}

fn render_return_body(
    return_type: &Type,
    result_name: &str,
    return_sink: &str,
) -> String {
    match return_type {
        Type::Primitive(PrimitiveType::Number) => format!(
            "                return {{ kind: \"float\", value: {result_name} }};",
        ),
        Type::Primitive(PrimitiveType::String) => format!(
            "                return {{ kind: \"string\", value: {result_name} }};",
        ),
        Type::Primitive(PrimitiveType::Boolean) => format!(
            "                return {{ kind: \"boolean\", value: {result_name} }};",
        ),
        Type::Composite(_) => render_reference_return_body(result_name, return_sink),
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => format!(
            "                return {{ kind: \"string\", value: JSON.stringify({result_name}) }};",
        ),
    }
}

fn render_reference_return_body(result_name: &str, return_sink: &str) -> String {
    format!(
        "                instanceRegistry.set({return_sink}, {result_name});\n                return {{ kind: \"reference\", value: {return_sink} }};",
    )
}

fn render_request_body(property_type: &Type, accessor: &str) -> String {
    let value_expression = format!("typedParent.{accessor}");
    match property_type {
        Type::Primitive(PrimitiveType::Number) => format!(
            "                return {{ kind: \"float\", value: {value_expression} }};",
        ),
        Type::Primitive(PrimitiveType::String) => format!(
            "                return {{ kind: \"string\", value: {value_expression} }};",
        ),
        Type::Primitive(PrimitiveType::Boolean) => format!(
            "                return {{ kind: \"boolean\", value: {value_expression} }};",
        ),
        Type::Composite(_) => format!(
            "                const newReference = crypto.randomUUID();\n                instanceRegistry.set(newReference, {value_expression});\n                return {{ kind: \"reference\", value: newReference }};",
        ),
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => format!(
            "                return {{ kind: \"string\", value: JSON.stringify({value_expression}) }};",
        ),
    }
}

fn render_update_body(accessor: &str, value_expression: &str) -> String {
    format!(
        "                typedParent.{accessor} = {value_expression};\n                return;",
    )
}

fn module_namespace_name(module_path: &ModulePath) -> String {
    if module_path.segments.is_empty() {
        return "library_root".to_string();
    }
    format!(
        "library_{}",
        module_path
            .segments
            .iter()
            .map(|segment| sanitize_identifier(segment))
            .collect::<Vec<_>>()
            .join("_")
    )
}

fn sanitize_identifier(input: &str) -> String {
    let mut result = String::new();
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            result.push(character);
        } else {
            result.push('_');
        }
    }
    if result.is_empty() {
        "_".to_string()
    } else if result
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        format!("_{}", result)
    } else {
        result
    }
}

fn module_import_path(from_module: &ModulePath, target_module: &ModulePath) -> String {
    let mut from_segments = vec!["library".to_string()];
    from_segments.extend(from_module.segments.iter().cloned());
    let mut target_segments = vec!["library".to_string()];
    target_segments.extend(target_module.segments.iter().cloned());
    target_segments.push("index.ts".to_string());
    let mut common_prefix = 0usize;
    while common_prefix < from_segments.len()
        && common_prefix < target_segments.len() - 1
        && from_segments[common_prefix] == target_segments[common_prefix]
    {
        common_prefix += 1;
    }
    let mut path = String::new();
    for _ in common_prefix..from_segments.len() {
        path.push_str("../");
    }
    let remainder = target_segments[common_prefix..].join("/");
    if path.is_empty() {
        path.push_str("./");
    }
    path.push_str(&remainder);
    path
}

#[cfg(test)]
mod tests {
    use crate::features::TypePath;

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
    fn generate_callee_renders_runtime_and_library_outputs() {
        let module = Module {
            path: ModulePath::empty(),
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
                    return_type: Type::Composite(TypePath::new(
                        ModulePath::empty(),
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

        let outputs = generate_callee(vec![&module]);

        assert!(
            outputs
                .iter()
                .any(|output| output.path == PathBuf::from("main.ts"))
        );
        assert!(
            outputs
                .iter()
                .any(|output| output.path == PathBuf::from("dispatch.ts"))
        );

        let dispatch_ts = outputs
            .iter()
            .find(|output| output.path == PathBuf::from("dispatch.ts"))
            .expect("dispatch.ts should be generated");
        assert!(dispatch_ts.content.contains("dispatchMessage"));
        assert!(dispatch_ts.content.contains("trim_whitespace"));
        assert!(dispatch_ts.content.contains("Point"));
        assert!(!dispatch_ts.content.contains("{{"));
    }
}
