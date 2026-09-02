use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::{FunctionDefinition, Method, Module, PrimitiveType, Type, TypeDefinition, TypeParameter},
};

const CALLEE_BRIDGE: &str = include_str!("./assets/bridge.hpp");
const CALLEE_SOCKET: &str = include_str!("./assets/socket.hpp");
const CALLEE_MAIN: &str = include_str!("./assets/callee_main.hpp");
const CALLEE_MAIN_ENTRYPOINT: &str = "#include \"main.hpp\"\n\nint main() {\n    return ptip_ffi::run_library_server();\n}\n";
const CALLEE_DISPATCH: &str = include_str!("./assets/callee_dispatch.hpp");

pub fn generate_callee(modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");
    let declarations = render_declarations(&modules);
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

fn render_declarations(modules: &[&Module]) -> String {
    let mut function_cases = Vec::new();
    let mut method_cases = Vec::new();
    for module in modules {
        for function in &module.functions {
            function_cases.push(render_function_case(function));
        }
        for definition in &module.types {
            if let Some(constructor) = &definition.default_constructor {
                function_cases.push(render_constructor_case(definition, constructor));
            } else {
                function_cases.push(render_constructor_case(
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
                method_cases.push(render_method_case(definition, method));
            }
        }
    }
    format!(
        "    if (kind == \"C\") {{\n        if (lines.size() < 3) return \"\";\n        const std::string target = decode_target_name(lines[1]);\n        const std::string return_sink = lines[2];\n{}\n        throw std::runtime_error(\"unknown function target: \" + target);\n    }}\n\n    if (kind == \"M\") {{\n        if (lines.size() < 4) return \"\";\n        const std::string called_reference = lines[1];\n        const std::string method_name = decode_target_name(lines[2]);\n        const std::string return_sink = lines[3];\n        auto& entry = instance_registry().at(called_reference);\n{}\n        throw std::runtime_error(\"unsupported method: \" + method_name);\n    }}",
        function_cases.join("\n"),
        method_cases.join("\n"),
    )
}

fn render_function_case(function: &FunctionDefinition) -> String {
    let arguments = render_arguments(&function.callable.positional_parameters, &function.callable.type_parameters, 3);
    let invocation = format!("{}({})", function.name, arguments);
    render_call_case(&function.name, &function.callable.return_type, &invocation, &function.callable.type_parameters)
}

fn render_constructor_case(definition: &TypeDefinition, constructor: &crate::features::AnonymousCallable) -> String {
    let concrete_type = concrete_type_name(&definition.name, &definition.type_parameters);
    let arguments = render_arguments(&constructor.positional_parameters, &definition.type_parameters, 3);
    let invocation = format!("{}({})", concrete_type, arguments);
    format!(
        "    if (target == \"{name}\") {{\n        const auto instance = {invocation};\n        instance_registry().emplace(return_sink, instance);\n        return respond_with_reference(return_sink, return_sink);\n    }}",
        name = definition.name,
        invocation = invocation,
    )
}

fn render_method_case(definition: &TypeDefinition, method: &Method) -> String {
    let concrete_type = concrete_type_name(&definition.name, &definition.type_parameters);
    let arguments = render_arguments(&method.callable.positional_parameters, &definition.type_parameters, 4);
    let invocation = format!("instance->{}({})", method.name, arguments);
    let result = render_return_expression(&method.callable.return_type, "result", "return_sink", &definition.type_parameters);
    format!(
        "    if (auto* instance = std::any_cast<{concrete_type}>(&entry)) {{\n        if (method_name == \"{method_name}\") {{\n{call}\n        }}\n    }}",
        concrete_type = concrete_type,
        method_name = method.name,
        call = render_method_invocation(&method.callable.return_type, &invocation, &result),
    )
}

fn render_method_invocation(return_type: &Type, invocation: &str, result: &str) -> String {
    if matches!(return_type, Type::Dynamic) {
        return format!("            {invocation};\n            return respond_with_value(return_sink, ptip_ffi::encode_value(std::string(\"undefined\")));");
    }
    format!("            const auto result = {invocation};\n{result}")
}

fn render_call_case(name: &str, return_type: &Type, invocation: &str, type_parameters: &[TypeParameter]) -> String {
    let arguments = if matches!(return_type, Type::Dynamic) {
        format!("        {invocation};\n        return respond_with_value(return_sink, ptip_ffi::encode_value(std::string(\"undefined\")));")
    } else {
        let result = render_return_expression(return_type, "result", "return_sink", type_parameters);
        format!("        const auto result = {invocation};\n{result}")
    };
    let _ = type_parameters;
    format!("    if (target == \"{name}\") {{\n{arguments}\n    }}")
}

fn render_return_expression(return_type: &Type, result_name: &str, sink_name: &str, type_parameters: &[TypeParameter]) -> String {
    if let Type::Composite(path) = return_type
        && type_parameters.iter().any(|parameter| parameter.name == path.name)
    {
        return format!("            return respond_with_value({sink_name}, ptip_ffi::encode_value({result_name}));");
    }
    match return_type {
        Type::Primitive(PrimitiveType::Number)
        | Type::Primitive(PrimitiveType::String)
        | Type::Primitive(PrimitiveType::Boolean) => {
            format!("            return respond_with_value({sink_name}, ptip_ffi::encode_value({result_name}));")
        }
        Type::Dynamic => format!("            return respond_with_value({sink_name}, ptip_ffi::encode_value(std::string(\"undefined\")));"),
        Type::Composite(_) | Type::Pointer(_) => format!(
            "            instance_registry().emplace({sink_name}, {result_name});\n            return respond_with_reference({sink_name}, {sink_name});",
            sink_name = sink_name,
            result_name = result_name,
        ),
        Type::Array(_) | Type::Tuple(_) => "            return ptip_ffi::encode_value(std::string(\"unsupported\"));".to_string(),
    }
}

fn render_arguments(parameters: &[crate::features::ValueParameter], type_parameters: &[TypeParameter], line_offset: usize) -> String {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let concrete = render_type(&parameter.r#type, type_parameters);
            if matches!(parameter.r#type, Type::Primitive(_)) || matches!(concrete.as_str(), "double" | "std::string" | "bool" | "int") {
                format!("ptip_ffi::decode_value<{}>(ptip_ffi::decode_parameter_line(lines[{}]))", concrete, index + line_offset)
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
