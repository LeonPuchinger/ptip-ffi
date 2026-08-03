mod codegen;
mod parser;

use crate::config::LanguageConfig;

pub fn register() -> LanguageConfig {
    LanguageConfig {
        name: "TypeScript",
        parse: parser::parse,
        generate_caller: codegen::generate_caller,
        generate_callee: codegen::generate_callee,
    }
}
