use std::path::PathBuf;

use crate::{
    codegen::{template::TemplateEngine, CodegenOutput},
    features::Module,
    map,
};

const CALLEE_INIT: &str = include_str!("./assets/callee/__init__.py");
const FFI_BRIDGE: &str = include_str!("./assets/bridge.py");
const FFI_SOCKET: &str = include_str!("./assets/socket.py");
const CALLEE_DISPATCH_TEMPLATE: &str = include_str!("./assets/callee/dispatch.py");
const CALLEE_MAIN_TEMPLATE: &str = include_str!("./assets/callee/main.py");
const CALL_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/call_handler.py");
const METHOD_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/method_handler.py");
const REQUEST_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/request_handler.py");
const UPDATE_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/update_handler.py");
const DROP_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/drop_handler.py");
const ERROR_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/error_handler.py");
const ACKNOWLEDGE_HANDLER_TEMPLATE: &str = include_str!("./assets/callee/acknowledge_handler.py");

pub fn generate_callee(_modules: Vec<&Module>) -> Vec<CodegenOutput> {
    let engine = TemplateEngine::new("{{NAME}}").expect("failed to compile template placeholder");

    vec![
        CodegenOutput {
            path: PathBuf::from("__init__.py"),
            content: CALLEE_INIT.to_string(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/bridge.py"),
            content: FFI_BRIDGE.to_string(),
        },
        CodegenOutput {
            path: PathBuf::from("ffi/socket.py"),
            content: FFI_SOCKET.to_string(),
        },
        CodegenOutput {
            path: PathBuf::from("dispatch.py"),
            content: render_dispatch(&engine),
        },
        CodegenOutput {
            path: PathBuf::from("main.py"),
            content: render_main(&engine),
        },
    ]
}

fn render_dispatch(engine: &TemplateEngine) -> String {
    let handlers = [
        CALL_HANDLER_TEMPLATE,
        METHOD_HANDLER_TEMPLATE,
        REQUEST_HANDLER_TEMPLATE,
        UPDATE_HANDLER_TEMPLATE,
        DROP_HANDLER_TEMPLATE,
        ERROR_HANDLER_TEMPLATE,
        ACKNOWLEDGE_HANDLER_TEMPLATE,
    ]
    .into_iter()
    .map(|template| engine.render(template, &map! {}, false))
    .collect::<Vec<_>>()
    .join("\n");

    engine.render(
        CALLEE_DISPATCH_TEMPLATE,
        &map! {
            "HANDLERS" => handlers.as_str(),
        },
        true,
    )
}

fn render_main(engine: &TemplateEngine) -> String {
    engine.render(CALLEE_MAIN_TEMPLATE, &map! {}, false)
}
