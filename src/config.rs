use crate::{
    features::LanguageFeature,
    parser::{ParserError, lexer::Lexer},
};

pub struct LanguageConfig {
    pub name: &'static str,
    pub build_lexer: for<'a> fn(&'a str) -> Box<dyn Lexer<'a> + 'a>,
    pub parser: for<'a> fn(&mut dyn Lexer<'a>) -> Result<Vec<LanguageFeature>, ParserError>,
}
