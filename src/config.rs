use crate::{
    features::LanguageFeature,
    parser::{
        ParserError,
        lexer::{Lexer, LexerError},
    },
};

pub struct LanguageConfig {
    pub name: &'static str,
    pub build_lexer: for<'a> fn(&'a str) -> Result<Box<dyn Lexer<'a> + 'a>, LexerError>,
    pub parser: for<'a> fn(&mut dyn Lexer<'a>) -> Result<Vec<LanguageFeature>, ParserError>,
}
