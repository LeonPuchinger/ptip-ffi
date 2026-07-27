use crate::{features::Module, parser::ParserError};

pub struct LanguageConfig {
    pub name: &'static str,
    pub parse: for<'a> fn(&'a str) -> Result<Module, ParserError>,
}
