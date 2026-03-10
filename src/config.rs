use crate::{
    features::LanguageFeature,
    parser::{Parser, lexer::Lexer},
};

pub struct LanguageConfig<'a> {
    pub name: &'a str,
    pub build_lexer: Box<dyn Fn(&'a str) -> Box<dyn Lexer<'a>> + Send + Sync + 'a>,
    pub parser: Parser<'a, Vec<LanguageFeature>>,
}
