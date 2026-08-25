mod codegen_callee;
mod codegen_caller;
mod parser;

use crate::config::LanguageConfig;

pub fn register() -> LanguageConfig {
    LanguageConfig {
        name: "Python",
        parse: parser::parse,
        generate_caller: codegen_caller::generate_caller,
        generate_callee: codegen_callee::generate_callee,
    }
}