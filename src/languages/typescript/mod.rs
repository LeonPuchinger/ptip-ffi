mod codegen;
mod parser;

use crate::config::LanguageConfig;

pub fn register() -> LanguageConfig {
    LanguageConfig {
        name: "TypeScript",
        parse: parser::parse,
        generate: codegen::generate,
    }
}
