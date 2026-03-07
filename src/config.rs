use crate::parser::{
    Parser,
    lexer::{LazyLexer, Lexer},
};

pub enum LanguageFeature {
    Function,
    Type,
}

pub struct FeatureParser<'a, L: Lexer<'a>> {
    pub(crate) kind: LanguageFeature,
    pub(crate) lexer: L,
    pub(crate) parser: Parser<'a, String>,
}

pub struct LanguageConfig<'a> {
    pub name: &'a str,
    pub features: Vec<FeatureParser<'a, LazyLexer<'a>>>,
}
