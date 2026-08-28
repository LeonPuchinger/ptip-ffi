use std::path::PathBuf;

use crate::{
    codegen::{CodegenOutput, template::TemplateEngine},
    features::Module,
};

const CALLEE_BRIDGE: &str = include_str!("./assets/bridge.hpp");
const CALLEE_SOCKET: &str = include_str!("./assets/socket.hpp");
const CALLEE_MAIN: &str = include_str!("./assets/callee_main.hpp");
const CALLEE_DISPATCH: &str = include_str!("./assets/caller_stub.hpp");

pub fn generate_callee(_modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");
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
                    "NAME" => "ptip_ffi_dispatch",
                    "STUBS" => "",
                },
                false,
            ),
        },
    ]
}
