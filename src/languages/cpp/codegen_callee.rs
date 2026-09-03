use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{FunctionDefinition, Method, Module, PrimitiveType, Type, TypeDefinition, TypeParameter},
};

const CALLEE_BRIDGE: &str = include_str!("./assets/bridge.hpp");
const CALLEE_SOCKET: &str = include_str!("./assets/socket.hpp");
const CALLEE_MAIN: &str = include_str!("./assets/callee_main.hpp");
const CALLEE_MAIN_ENTRYPOINT: &str = include_str!("./assets/callee_main.cpp");
const CALLEE_DISPATCH: &str = include_str!("./assets/callee_dispatch.hpp");
const CALLEE_DISPATCH_CASES: &str = include_str!("./assets/callee_dispatch_cases.hpp");
const CALLEE_FUNCTION_CASE: &str = include_str!("./assets/callee/function_case.hpp");
const CALLEE_FUNCTION_VOID_CASE: &str = include_str!("./assets/callee/function_void_case.hpp");
const CALLEE_CONSTRUCTOR_CASE: &str = include_str!("./assets/callee/constructor_case.hpp");
const CALLEE_METHOD_CASE: &str = include_str!("./assets/callee/method_case.hpp");
const CALLEE_METHOD_VOID_BODY: &str = include_str!("./assets/callee/method_void_body.hpp");
const CALLEE_METHOD_VALUE_BODY: &str = include_str!("./assets/callee/method_value_body.hpp");
const CALLEE_RETURN_VALUE: &str = include_str!("./assets/callee/return_value.hpp");
const CALLEE_RETURN_REFERENCE: &str = include_str!("./assets/callee/return_reference.hpp");
const CALLEE_RETURN_UNDEFINED: &str = include_str!("./assets/callee/return_undefined.hpp");
const CALLEE_DECODE_VALUE: &str = include_str!("./assets/callee/decode_value.hpp");

pub fn generate_callee(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");
    let declarations = render_declarations(&engine, &modules);
    vec![
        CodegenOutput {
            path: PathBuf::from("bridge.hpp"),
            content: CALLEE_BRIDGE.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("socket.hpp"),
            content: CALLEE_SOCKET.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("main.hpp"),
            content: CALLEE_MAIN.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("main.cpp"),
            content: CALLEE_MAIN_ENTRYPOINT.to_owned(),
        },
        CodegenOutput {
            path: PathBuf::from("dispatch.hpp"),
            content: engine.render(
                CALLEE_DISPATCH,
                &crate::map! {
                    "DISPATCH_CASES" => declarations.as_str(),
                },
                false,
            ),
        },
    ]
}

fn render_declarations(engine: &TemplateEngine, modules: &[&Module]) -> String {
    let mut function_cases = Vec::new();
    let mut method_cases = Vec::new();
    for module in modules {
        for function in &module.functions {
            function_cases.push(render_function_case(engine, function));
        }
        for definition in &module.types {
            if let Some(constructor) = &definition.default_constructor {
                function_cases.push(render_constructor_case(engine, definition, constructor));
            } else {
                function_cases.push(render_constructor_case(
                    engine,
                    definition,
                    &crate::features::AnonymousCallable {
                        positional_parameters: Vec::new(),
                        named_parameters: Vec::new(),
                        return_type: Type::Dynamic,
                        type_parameters: definition.type_parameters.clone(),
                    },
                ));
            }
            for method in &definition.methods {
                method_cases.push(render_method_case(engine, definition, method));
            }
        }
    }
    let function_cases = function_cases.join("\n");
    let method_cases = method_cases.join("\n");
    engine.render(
        CALLEE_DISPATCH_CASES,
        &crate::map! {
            "FUNCTION_CASES" => function_cases.as_str(),
            "METHOD_CASES" => method_cases.as_str(),
        },
        true,
    )
}

fn render_function_case(engine: &TemplateEngine, function: &FunctionDefinition) -> String {
    let arguments = render_arguments(engine, &function.callable.positional_parameters, &function.callable.type_parameters, 3);
    let invocation = format!("{}({})", function.name, arguments);
    render_call_case(engine, &function.name, &function.callable.return_type, &invocation, &function.callable.type_parameters)
}

fn render_constructor_case(engine: &TemplateEngine, definition: &TypeDefinition, constructor: &crate::features::AnonymousCallable) -> String {
    let concrete_type = concrete_type_name(&definition.name, &definition.type_parameters);
    let arguments = render_arguments(engine, &constructor.positional_parameters, &definition.type_parameters, 3);
    let invocation = format!("{}({})", concrete_type, arguments);
    engine.render(
        CALLEE_CONSTRUCTOR_CASE,
        &crate::map! {
            "NAME" => definition.name.as_str(),
            "INVOCATION" => invocation.as_str(),
        },
        true,
    )
}

fn render_method_case(engine: &TemplateEngine, definition: &TypeDefinition, method: &Method) -> String {
    let concrete_type = concrete_type_name(&definition.name, &definition.type_parameters);
    let arguments = render_arguments(engine, &method.callable.positional_parameters, &definition.type_parameters, 4);
    let invocation = format!("instance->{}({})", method.name, arguments);
    let body = render_method_invocation(engine, &method.callable.return_type, &invocation, &definition.type_parameters);
    engine.render(
        CALLEE_METHOD_CASE,
        &crate::map! {
            "TYPE" => concrete_type.as_str(),
            "NAME" => method.name.as_str(),
            "BODY" => body.as_str(),
        },
        true,
    )
}

fn render_method_invocation(engine: &TemplateEngine, return_type: &Type, invocation: &str, type_parameters: &[TypeParameter]) -> String {
    if matches!(return_type, Type::Dynamic) {
        let return_body = render_return_expression(engine, return_type, "result", "return_sink", type_parameters);
        return engine.render(
            CALLEE_METHOD_VOID_BODY,
            &crate::map! {
                "INVOCATION" => invocation,
                "RETURN" => return_body.as_str(),
            },
            true,
        );
    }
    let return_body = render_return_expression(engine, return_type, "result", "return_sink", type_parameters);
    engine.render(
        CALLEE_METHOD_VALUE_BODY,
        &crate::map! {
            "INVOCATION" => invocation,
            "RETURN" => return_body.as_str(),
        },
        true,
    )
}

fn render_call_case(engine: &TemplateEngine, name: &str, return_type: &Type, invocation: &str, type_parameters: &[TypeParameter]) -> String {
    if matches!(return_type, Type::Dynamic) {
        let return_body = render_return_expression(engine, return_type, "result", "return_sink", type_parameters);
        return engine.render(
            CALLEE_FUNCTION_VOID_CASE,
            &crate::map! {
                "NAME" => name,
                "INVOCATION" => invocation,
                "RETURN" => return_body.as_str(),
            },
            true,
        );
    }
    let result = render_return_expression(engine, return_type, "result", "return_sink", type_parameters);
    engine.render(
        CALLEE_FUNCTION_CASE,
        &crate::map! {
            "NAME" => name,
            "INVOCATION" => invocation,
            "RETURN" => result.as_str(),
        },
        true,
    )
}

fn render_return_expression(engine: &TemplateEngine, return_type: &Type, result_name: &str, sink_name: &str, type_parameters: &[TypeParameter]) -> String {
    if let Type::Composite(path) = return_type
        && type_parameters.iter().any(|parameter| parameter.name == path.name)
    {
        return engine.render(CALLEE_RETURN_VALUE, &crate::map! { "SINK" => sink_name, "VALUE" => result_name }, false);
    }
    match return_type {
        Type::Primitive(PrimitiveType::Number)
        | Type::Primitive(PrimitiveType::String)
        | Type::Primitive(PrimitiveType::Boolean) => {
            engine.render(CALLEE_RETURN_VALUE, &crate::map! { "SINK" => sink_name, "VALUE" => result_name }, false)
        }
        Type::Dynamic => engine.render(CALLEE_RETURN_UNDEFINED, &crate::map! { "SINK" => sink_name }, false),
        Type::Composite(_) | Type::Pointer(_) => engine.render(CALLEE_RETURN_REFERENCE, &crate::map! { "SINK" => sink_name, "VALUE" => result_name }, false),
        Type::Array(_) | Type::Tuple(_) => engine.render(CALLEE_RETURN_VALUE, &crate::map! { "SINK" => sink_name, "VALUE" => "std::string(\"unsupported\")" }, false),
    }
}

fn render_arguments(engine: &TemplateEngine, parameters: &[crate::features::ValueParameter], type_parameters: &[TypeParameter], line_offset: usize) -> String {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let concrete = render_type(&parameter.r#type, type_parameters);
            if matches!(parameter.r#type, Type::Primitive(_)) || matches!(concrete.as_str(), "double" | "std::string" | "bool" | "int") {
                let index = (index + line_offset).to_string();
                engine.render(CALLEE_DECODE_VALUE, &crate::map! { "TYPE" => concrete.as_str(), "INDEX" => index.as_str() }, false)
            } else if matches!(parameter.r#type, Type::Pointer(_)) {
                format!("decode_reference_pointer<{}>(ptip_ffi::decode_parameter_line(lines[{}]).value)", concrete.trim_end_matches('*'), index + line_offset)
            } else {
                format!("decode_reference<{}>(ptip_ffi::decode_parameter_line(lines[{}]).value)", concrete.trim_end_matches('*'), index + line_offset)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_type(r#type: &Type, type_parameters: &[TypeParameter]) -> String {
    match r#type {
        Type::Primitive(PrimitiveType::Number) => "double".to_string(),
        Type::Primitive(PrimitiveType::String) => "std::string".to_string(),
        Type::Primitive(PrimitiveType::Boolean) => "bool".to_string(),
        Type::Dynamic => "std::any".to_string(),
        Type::Pointer(inner) => format!("{}*", render_type(inner, type_parameters)),
        Type::Array(inner) => format!("std::vector<{}>", render_type(inner, type_parameters)),
        Type::Tuple(elements) => format!("std::tuple<{}>", elements.iter().map(|element| render_type(element, type_parameters)).collect::<Vec<_>>().join(", ")),
        Type::Composite(path) => {
            if let Some(parameter) = type_parameters.iter().find(|parameter| parameter.name == path.name) {
                return concrete_type_parameter(&parameter.name);
            }
            let arguments = path.type_arguments.iter().map(|argument| render_type(argument, type_parameters)).collect::<Vec<_>>();
            if arguments.is_empty() { path.name.clone() } else { format!("{}<{}>", path.name, arguments.join(", ")) }
        }
    }
}

fn concrete_type_parameter(name: &str) -> String {
    match name {
        "K" => "std::string".to_string(),
        _ => "int".to_string(),
    }
}

fn concrete_type_name(name: &str, parameters: &[TypeParameter]) -> String {
    if parameters.is_empty() { return name.to_string(); }
    format!("{}<{}>", name, parameters.iter().map(|parameter| concrete_type_parameter(&parameter.name)).collect::<Vec<_>>().join(", "))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::generate_callee;
    use crate::features::{
        AnonymousCallable, FunctionDefinition, Module, ModulePath, PrimitiveType, Type,
        ValueParameter,
    };

    #[test]
    fn generate_callee_emits_runtime_files() {
        let module = Module {
            path: ModulePath::empty(),
            functions: vec![FunctionDefinition {
                name: "identity".to_string(),
                callable: AnonymousCallable {
                    positional_parameters: vec![ValueParameter {
                        name: "value".to_string(),
                        r#type: Type::Primitive(PrimitiveType::Number),
                        required: true,
                        variadic: false,
                        nullable: false,
                    }],
                    named_parameters: Vec::new(),
                    return_type: Type::Primitive(PrimitiveType::Number),
                    type_parameters: Vec::new(),
                },
            }],
            types: Vec::new(),
        };

        let outputs = generate_callee(vec![&module]);
        let dispatch = outputs
            .iter()
            .find(|output| output.path == Path::new("dispatch.hpp"))
            .expect("dispatch.hpp should be generated");
        let main_entrypoint = outputs
            .iter()
            .find(|output| output.path == Path::new("main.cpp"))
            .expect("main.cpp should be generated");

        assert!(!dispatch.content.is_empty());
        assert!(dispatch.content.contains("namespace ptip_ffi_generated"));
        assert!(dispatch.content.contains("target == \"identity\""));
        assert!(main_entrypoint.content.contains("int main()"));
        assert!(main_entrypoint.content.contains("run_library_server"));

        let main_header = outputs
            .iter()
            .find(|output| output.path == Path::new("main.hpp"))
            .expect("main.hpp should be generated");
        assert!(main_header.content.contains("UnixDomainListener listener(socket_path);"));
        assert!(main_header.content.contains("std::cout << socket_path"));
        assert!(main_header.content.contains("socket_path"));
        assert!(!main_header.content.contains("FFI_LIBRARY_INVOKE"));
        assert!(!main_header.content.contains("resolve_library_path"));
    }
}
