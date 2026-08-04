use std::{collections::BTreeSet, path::PathBuf};

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{
        AnonymousCallable, FunctionDefinition, Method, Module, ModulePath, PrimitiveType, Type,
        TypeDefinition, TypeParameter, TypePath, ValueParameter,
    },
};

const SOCKET_IMPLEMENTATION: &str = include_str!("./assets/socket.ts");
const BRIDGE_IMPLEMENTATION: &str = include_str!("./assets/bridge.ts");

const CALLEE_PACKAGE_JSON: &str = include_str!("./assets/callee/package.json");
const CALLEE_PACKAGE_LOCK_JSON: &str = include_str!("./assets/callee/package-lock.json");
const CALLEE_MAIN: &str = include_str!("./assets/callee/main.ts");
const CALLEE_DISPATCH: &str = include_str!("./assets/callee/dispatch.ts");
const CALLEE_LIBRARY_INDEX: &str = include_str!("./assets/callee/library/index.ts");
const CALLEE_FUNCTION_DECLARATION: &str = include_str!("./assets/callee/function.ts");
const CALLEE_CLASS_DECLARATION: &str = include_str!("./assets/callee/class.ts");
const CALLEE_CONSTRUCTOR_DECLARATION: &str = include_str!("./assets/callee/constructor.ts");
const CALLEE_METHOD_DECLARATION: &str = include_str!("./assets/callee/method.ts");
const CALLEE_STATIC_METHOD_DECLARATION: &str = include_str!("./assets/callee/static_method.ts");
const CALLEE_STATIC_NAMED_CONSTRUCTOR_DECLARATION: &str =
    include_str!("./assets/callee/static_named_constructor.ts");
const CALLEE_PROPERTY_DECLARATION: &str = include_str!("./assets/callee/property.ts");
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
    let rendered_modules = render_callee_modules(&engine, &modules);
    let rendered_dispatch = render_dispatch(&engine, &modules);
    let mut outputs = vec![
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
    ];
    outputs.extend(rendered_modules);
    outputs
}

fn render_callee_modules(engine: &TemplateEngine, modules: &[&Module]) -> Vec<CodegenOutput> {
    modules
        .iter()
        .map(|module| CodegenOutput {
            path: callee_module_output_path(&module.path),
            content: render_callee_module(engine, module, modules),
        })
        .collect()
}

fn render_callee_module(
    engine: &TemplateEngine,
    module: &Module,
    all_modules: &[&Module],
) -> String {
    let imports = render_module_imports(module, all_modules);
    let declarations = render_module_declarations(engine, module);
    engine.render(
        CALLEE_LIBRARY_INDEX,
        &crate::map! {
            "IMPORTS" => imports.as_str(),
            "DECLARATIONS" => declarations.as_str(),
        },
        false,
    )
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

fn render_module_imports(module: &Module, all_modules: &[&Module]) -> String {
    let mut dependencies = BTreeSet::new();
    for function in &module.functions {
        collect_callable_dependencies(&function.callable, &module.path, &mut dependencies);
    }
    for definition in &module.types {
        collect_type_definition_dependencies(definition, &module.path, &mut dependencies);
    }
    let mut imports = Vec::new();
    for dependency in dependencies {
        if dependency == module.path.format("/") {
            continue;
        }

        if let Some(target) = all_modules
            .iter()
            .find(|candidate| candidate.path.format("/") == dependency)
        {
            imports.push(format!(
                "import * as {} from \"{}\";",
                module_namespace_name(&target.path),
                module_import_path(&module.path, &target.path)
            ));
        }
    }
    imports.join("\n")
}

fn render_dispatch_imports(modules: &[&Module]) -> String {
    modules
        .iter()
        .map(|module| {
            format!(
                "import * as {} from \"{}\";",
                module_namespace_name(&module.path),
                module_import_path(&ModulePath::empty(), &module.path)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_module_declarations(engine: &TemplateEngine, module: &Module) -> String {
    let mut declarations = Vec::new();
    for function in &module.functions {
        declarations.push(render_function_declaration(engine, function, &module.path));
    }
    for definition in &module.types {
        declarations.push(render_type_definition(engine, definition, &module.path));
    }
    declarations.join("\n\n")
}

fn render_function_declaration(
    engine: &TemplateEngine,
    function: &FunctionDefinition,
    current_module: &ModulePath,
) -> String {
    let parameters = render_parameters(&function.callable.positional_parameters, current_module);
    let type_parameters =
        render_type_parameters(&function.callable.type_parameters, current_module);
    let return_type = render_type_annotation(&function.callable.return_type, current_module);
    let body = render_not_implemented_body();
    engine.render(
        CALLEE_FUNCTION_DECLARATION,
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

fn render_type_definition(
    engine: &TemplateEngine,
    definition: &TypeDefinition,
    current_module: &ModulePath,
) -> String {
    let type_parameters = render_type_parameters(&definition.type_parameters, current_module);
    let implements = render_implements_clause(&definition.implements, current_module);
    let mut members = Vec::new();
    if let Some(constructor) = &definition.default_constructor {
        members.push(render_constructor(engine, constructor, current_module));
    }
    for property in &definition.properties {
        members.push(render_property(engine, property, current_module));
    }
    for method in &definition.methods {
        members.push(render_method(engine, method, current_module));
    }
    for method in &definition.static_methods {
        members.push(render_static_method(engine, method, current_module));
    }
    for constructor in &definition.named_constructors {
        members.push(render_named_constructor(
            engine,
            constructor,
            current_module,
            &definition.name,
        ));
    }
    let members = members.join("\n");
    engine.render(
        CALLEE_CLASS_DECLARATION,
        &crate::map! {
            "NAME" => definition.name.as_str(),
            "TYPE_PARAMETERS" => type_parameters.as_str(),
            "IMPLEMENTS" => implements.as_str(),
            "MEMBERS" => members.as_str(),
        },
        false,
    )
}

fn render_constructor(
    engine: &TemplateEngine,
    constructor: &AnonymousCallable,
    current_module: &ModulePath,
) -> String {
    let parameters = render_parameters(&constructor.positional_parameters, current_module);
    let body = render_not_implemented_body();
    engine.render(
        CALLEE_CONSTRUCTOR_DECLARATION,
        &crate::map! {
            "PARAMETERS" => parameters.as_str(),
            "BODY" => body.as_str(),
        },
        false,
    )
}

fn render_property(
    engine: &TemplateEngine,
    property: &(String, Type),
    current_module: &ModulePath,
) -> String {
    let return_type = render_type_annotation(&property.1, current_module);
    engine.render(
        CALLEE_PROPERTY_DECLARATION,
        &crate::map! {
            "NAME" => property.0.as_str(),
            "RETURN_TYPE" => return_type.as_str(),
        },
        false,
    )
}

fn render_method(engine: &TemplateEngine, method: &Method, current_module: &ModulePath) -> String {
    let parameters = render_parameters(&method.callable.positional_parameters, current_module);
    let type_parameters = render_type_parameters(&method.callable.type_parameters, current_module);
    let return_type = render_type_annotation(&method.callable.return_type, current_module);
    let body = render_not_implemented_body();
    engine.render(
        CALLEE_METHOD_DECLARATION,
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

fn render_static_method(
    engine: &TemplateEngine,
    method: &Method,
    current_module: &ModulePath,
) -> String {
    let parameters = render_parameters(&method.callable.positional_parameters, current_module);
    let type_parameters = render_type_parameters(&method.callable.type_parameters, current_module);
    let return_type = render_type_annotation(&method.callable.return_type, current_module);
    let body = render_not_implemented_body();
    engine.render(
        CALLEE_STATIC_METHOD_DECLARATION,
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

fn render_named_constructor(
    engine: &TemplateEngine,
    constructor: &FunctionDefinition,
    current_module: &ModulePath,
    type_name: &str,
) -> String {
    let parameters = render_parameters(&constructor.callable.positional_parameters, current_module);
    let type_parameters =
        render_type_parameters(&constructor.callable.type_parameters, current_module);
    let return_type = render_type_annotation(
        &Type::Composite(TypePath::new(current_module.clone(), type_name.to_string())),
        current_module,
    );
    let body = render_not_implemented_body();
    engine.render(
        CALLEE_STATIC_NAMED_CONSTRUCTOR_DECLARATION,
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

fn render_method_case(
    engine: &TemplateEngine,
    module_namespace: &str,
    definition: &TypeDefinition,
    method: &Method,
    module_path: &ModulePath,
) -> String {
    let arguments = render_call_arguments(&method.callable.positional_parameters, module_path);
    let return_kind = render_result_kind(&method.callable.return_type);
    engine.render(
        CALLEE_METHOD_CASE,
        &crate::map! {
            "METHOD_NAME" => method.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "TYPE_NAME" => definition.name.as_str(),
            "ARGUMENTS" => arguments.as_str(),
            "RETURN_KIND" => return_kind,
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
    let return_kind = render_result_kind(&property.1);
    engine.render(
        CALLEE_REQUEST_CASE,
        &crate::map! {
            "ACCESSOR" => property.0.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "TYPE_NAME" => definition.name.as_str(),
            "RETURN_KIND" => return_kind,
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
    let value = render_parameter_decode_expression(&property.1, "value", current_module);
    engine.render(
        CALLEE_UPDATE_CASE,
        &crate::map! {
            "ACCESSOR" => property.0.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "TYPE_NAME" => definition.name.as_str(),
            "VALUE" => value.as_str(),
        },
        false,
    )
}

fn render_not_implemented_body() -> String {
    "        throw new Error(\"Not implemented\");".to_string()
}

fn render_implements_clause(implements: &[TypePath], current_module: &ModulePath) -> String {
    if implements.is_empty() {
        return String::new();
    }
    let rendered = implements
        .iter()
        .map(|r#type| render_type_annotation(&Type::Composite(r#type.clone()), current_module))
        .collect::<Vec<_>>()
        .join(", ");
    format!("implements {}", rendered)
}

fn render_parameters(parameters: &[ValueParameter], current_module: &ModulePath) -> String {
    parameters
        .iter()
        .map(|parameter| render_parameter_signature(parameter, current_module))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_parameter_signature(parameter: &ValueParameter, current_module: &ModulePath) -> String {
    let type_annotation = if parameter.variadic {
        format!(
            "{}[]",
            render_type_annotation(&parameter.r#type, current_module)
        )
    } else {
        render_type_annotation(&parameter.r#type, current_module)
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

fn render_type_parameters(parameters: &[TypeParameter], current_module: &ModulePath) -> String {
    if parameters.is_empty() {
        return String::new();
    }
    let rendered = parameters
        .iter()
        .map(|parameter| match &parameter.default {
            Some(default) => format!(
                "{} = {}",
                parameter.name,
                render_type_annotation(default, current_module)
            ),
            None => parameter.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{}>", rendered)
}

fn render_type_annotation(r#type: &Type, current_module: &ModulePath) -> String {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => "number".to_string(),
        Type::Primitive(PrimitiveType::String) => "string".to_string(),
        Type::Primitive(PrimitiveType::Boolean) => "boolean".to_string(),
        Type::Composite(path) => {
            if path.module_path.format("/") == current_module.format("/") {
                path.name.clone()
            } else {
                format!("{}.{}", module_namespace_name(&path.module_path), path.name)
            }
        }
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
    let return_kind = render_result_kind(&function.callable.return_type);
    let result_expression = format!("serializeValue(result, \"{}\", returnSink)", return_kind);
    engine.render(
        CALLEE_FUNCTION_CASE,
        &crate::map! {
            "CALLEE_NAME" => function.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
            "RESULT_EXPRESSION" => result_expression.as_str(),
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
    engine.render(
        CALLEE_CONSTRUCTOR_CASE,
        &crate::map! {
            "TYPE_NAME" => definition.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
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
    let return_kind = render_result_kind(&method.callable.return_type);
    let return_kind = return_kind.to_string();
    engine.render(
        CALLEE_STATIC_METHOD_CASE,
        &crate::map! {
            "METHOD_NAME" => method.name.as_str(),
            "TYPE_NAME" => definition.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
            "RETURN_KIND" => return_kind.as_str(),
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
    engine.render(
        CALLEE_STATIC_NAMED_CONSTRUCTOR_CASE,
        &crate::map! {
            "METHOD_NAME" => constructor.name.as_str(),
            "TYPE_NAME" => definition.name.as_str(),
            "MODULE_NAMESPACE" => module_namespace,
            "ARGUMENTS" => arguments.as_str(),
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
                "decodeParameter(positionalParameters[{index}], \"{}\") as {}",
                render_parameter_kind(&parameter.r#type),
                render_type_annotation(&parameter.r#type, current_module)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_parameter_decode_expression(
    r#type: &Type,
    name: &str,
    current_module: &ModulePath,
) -> String {
    format!(
        "decodeParameter({name}, \"{}\") as {}",
        render_parameter_kind(r#type),
        render_type_annotation(r#type, current_module)
    )
}

fn render_parameter_kind(r#type: &Type) -> &'static str {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => "number",
        Type::Primitive(PrimitiveType::String) => "string",
        Type::Primitive(PrimitiveType::Boolean) => "boolean",
        Type::Composite(_) => "reference",
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => "json",
    }
}

fn render_result_kind(r#type: &Type) -> &'static str {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => "number",
        Type::Primitive(PrimitiveType::String) => "string",
        Type::Primitive(PrimitiveType::Boolean) => "boolean",
        Type::Composite(_) => "reference",
        Type::Array(_) | Type::Tuple(_) | Type::Dynamic => "json",
    }
}

fn collect_callable_dependencies(
    callable: &AnonymousCallable,
    current_module: &ModulePath,
    dependencies: &mut BTreeSet<String>,
) {
    for parameter in &callable.positional_parameters {
        collect_type_dependencies(&parameter.r#type, current_module, dependencies);
    }
    collect_type_dependencies(&callable.return_type, current_module, dependencies);
    for parameter in &callable.type_parameters {
        if let Some(default) = &parameter.default {
            collect_type_dependencies(default, current_module, dependencies);
        }
    }
}

fn collect_type_definition_dependencies(
    definition: &TypeDefinition,
    current_module: &ModulePath,
    dependencies: &mut BTreeSet<String>,
) {
    for property in &definition.properties {
        collect_type_dependencies(&property.1, current_module, dependencies);
    }
    if let Some(constructor) = &definition.default_constructor {
        collect_callable_dependencies(constructor, current_module, dependencies);
    }
    for method in &definition.methods {
        collect_callable_dependencies(&method.callable, current_module, dependencies);
    }
    for method in &definition.static_methods {
        collect_callable_dependencies(&method.callable, current_module, dependencies);
    }
    for constructor in &definition.named_constructors {
        collect_callable_dependencies(&constructor.callable, current_module, dependencies);
    }
    for implemented in &definition.implements {
        if implemented.module_path.format("/") != current_module.format("/") {
            dependencies.insert(implemented.module_path.format("/"));
        }
    }
}

fn collect_type_dependencies(
    r#type: &Type,
    current_module: &ModulePath,
    dependencies: &mut BTreeSet<String>,
) {
    match r#type {
        Type::Primitive(_) | Type::Dynamic => {}
        Type::Composite(path) => {
            if path.module_path.format("/") != current_module.format("/") {
                dependencies.insert(path.module_path.format("/"));
            }
        }
        Type::Array(inner) => collect_type_dependencies(inner, current_module, dependencies),
        Type::Tuple(elements) => {
            for element in elements {
                collect_type_dependencies(element, current_module, dependencies);
            }
        }
    }
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

fn callee_module_output_path(module_path: &ModulePath) -> PathBuf {
    let mut path = PathBuf::from("library");
    for segment in &module_path.segments {
        path.push(segment);
    }
    path.push("index.ts");
    path
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
        assert!(
            outputs
                .iter()
                .any(|output| output.path == PathBuf::from("library/index.ts"))
        );

        let dispatch_ts = outputs
            .iter()
            .find(|output| output.path == PathBuf::from("dispatch.ts"))
            .expect("dispatch.ts should be generated");
        assert!(dispatch_ts.content.contains("dispatchMessage"));
        assert!(dispatch_ts.content.contains("trim_whitespace"));
        assert!(dispatch_ts.content.contains("Point"));
        assert!(!dispatch_ts.content.contains("{{"));

        let library_ts = outputs
            .iter()
            .find(|output| output.path == PathBuf::from("library/index.ts"))
            .expect("library/index.ts should be generated");
        assert!(
            library_ts
                .content
                .contains("export function trim_whitespace")
        );
        assert!(library_ts.content.contains("export class Point"));
        assert!(library_ts.content.contains("distance_to_origin(): number"));
        assert!(!library_ts.content.contains("{{"));
    }
}
