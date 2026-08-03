use crate::{codegen::CodegenOutput, features::Module, parser::ParserError};

pub struct LanguageConfig {
    pub name: &'static str,
    pub parse: for<'a> fn(&'a str) -> Result<Module, ParserError>,
    pub generate_caller: for<'a> fn(Vec<&'a Module>) -> Vec<CodegenOutput>,
    pub generate_callee: for<'a> fn(Vec<&'a Module>) -> Vec<CodegenOutput>,
}
