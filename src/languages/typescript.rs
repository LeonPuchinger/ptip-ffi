use std::vec;

use crate::{
    config::LanguageConfig,
    features::LanguageFeature,
    parser::{
        ParserError,
        atoms::{exact, token_kind},
        lexer::{self, LazyLexer, LexerDirective, LexerRule, StateModification},
    },
};

fn keyworded_function_definition(
    lexer: &mut dyn lexer::Lexer,
) -> Result<LanguageFeature, ParserError> {
    let _keyword = exact("function")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    let _open_parenthesis = exact("(")(lexer)?;
    let _close_parenthesis = exact(")")(lexer)?;
    Ok(LanguageFeature::Function {
        name,
        args: Vec::new(),
        return_type: String::new(),
    })
}

fn function_definitions(lexer: &mut dyn lexer::Lexer) -> Result<Vec<LanguageFeature>, ParserError> {
    let mut features = Vec::new();
    while let Ok(feature) = keyworded_function_definition(lexer) {
        features.push(feature);
    }
    Ok(features)
}

static SOME_LEXER_RULES: &[LexerDirective] = &[
    LexerDirective { rule: LexerRule { kind: "string_literal", pattern: r#""([^"\\]|\\.)*""# }, keep: true, modification: StateModification::PushLazy(&|| STRING_LITERALS) },
    LexerDirective { rule: LexerRule { kind: "string_literal", pattern: r#"'([^'\\]|\\.)*'"# }, keep: true, modification: StateModification::None },
];

static STRING_LITERALS: &[LexerDirective] = &[
    LexerDirective { rule: LexerRule { kind: "string_literal", pattern: r#""([^"\\]|\\.)*""# }, keep: true, modification: StateModification::Push(SOME_LEXER_RULES) },
    LexerDirective { rule: LexerRule { kind: "string_literal", pattern: r#"'([^'\\]|\\.)*'"# }, keep: true, modification: StateModification::None },
];

pub fn register() -> LanguageConfig {
    LanguageConfig {
        name: "TypeScript",
        build_lexer: |input| Box::new(LazyLexer::new(input, vec![
            LexerDirective { rule: LexerRule { kind: "whitespace", pattern: r"\s+" }, keep: false, modification: StateModification::None },
            LexerDirective { rule: LexerRule { kind: "keyword", pattern: r"\bfunction\b" }, keep: true, modification: StateModification::None },
            LexerDirective { rule: LexerRule { kind: "identifier", pattern: "[a-zA-Z_$][a-zA-Z0-9_$]*" }, keep: true, modification: StateModification::Push(STRING_LITERALS) },
        ])),
        parser: function_definitions,
    }
}
