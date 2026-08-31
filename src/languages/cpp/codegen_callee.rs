use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::Module,
};

const CALLEE_BRIDGE: &str = include_str!("./assets/bridge.hpp");
const CALLEE_SOCKET: &str = include_str!("./assets/socket.hpp");
const CALLEE_MAIN: &str = include_str!("./assets/callee_main.hpp");
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
            path: PathBuf::from("dispatch.hpp"),
            content: engine.render(
                CALLEE_DISPATCH,
                &crate::map! {
                    "DECLARATIONS" => declarations.as_str(),
                },
                false,
            ),
        },
    ]
}

fn render_declarations(modules: &[&Module]) -> String {
    let mut declarations = Vec::new();
    for module in modules {
        for function in &module.functions {
            declarations.push(format!("void dispatch_{}();", function.name));
        }
        for definition in &module.types {
            for method in &definition.methods {
                declarations.push(format!("void dispatch_{}_{}();", definition.name, method.name));
            }
            for method in &definition.static_methods {
                declarations.push(format!("void dispatch_{}_{}();", definition.name, method.name));
            }
        }
    }
    declarations.join("\n")
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

        assert!(!dispatch.content.is_empty());
        assert!(dispatch.content.contains("namespace ptip_ffi_generated"));
        assert!(dispatch.content.contains("dispatch_identity"));
    }
}
