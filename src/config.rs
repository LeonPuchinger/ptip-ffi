use crate::{
    features::LanguageFeature,
    parser::{
        Parser,
        lexer::{Lexer, LexerError},
    },
};

pub struct LanguageConfig<'p> {
    pub name: &'static str,
    pub build_lexer: for<'a> fn(&'a str) -> Result<Box<dyn Lexer<'a> + 'a>, LexerError>,
    pub parser: Parser<'p, Vec<LanguageFeature>>,
}
