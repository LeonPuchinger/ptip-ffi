use crate::{features::LanguageFeature, parser::ParserError};

pub struct LanguageConfig {
    pub name: &'static str,
    pub parse: for<'a> fn(&'a str) -> Result<Vec<LanguageFeature>, ParserError>,
}
