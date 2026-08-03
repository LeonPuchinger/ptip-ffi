use crate::{codegen::CodegenOutput, features::Module, parser::ParserError};

pub struct LanguageConfig {
    pub name: &'static str,
    pub parse: for<'a> fn(&'a str) -> Result<Module, ParserError>,
    pub generate: for<'a> fn(&'a Module) -> CodegenOutput,
}
