use crate::{
    config::LanguageConfig,
    features::LanguageFeature,
    map,
    parser::{
        Parser, ParserError,
        atoms::{exact, token_kind},
        combinators::{AnchorLocation, parse_at_anchors},
        lexer::{self, LazyStatefulLexer, Lexer, LexerRule, StateModification},
    },
};

fn keyworded_function_definition(lexer: &mut dyn Lexer) -> Result<LanguageFeature, ParserError> {
    exact("function")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    exact("(")(lexer)?;
    exact(")")(lexer)?;
    Ok(LanguageFeature::Function {
        name,
        args: Vec::new(),
        return_type: String::new(),
    })
}

// Root lexer ruleset for TypeScript. This ruleset tokenizes top-level
// statements (including function/type/interface/class declarations).
static STATEMENTS: &[LexerRule] = &[
    LexerRule {
        pattern: r"[ \t\r\n]+",
        kind: "whitespace",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"//[^\n]*",
        kind: "line_comment",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"(?s)/\*.*?\*/",
        kind: "block_comment",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\b(function|class)\b",
        kind: "keyword",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"[A-Za-z_$][A-Za-z0-9_$]*",
        kind: "identifier",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"[0-9]+(\.[0-9]+)?",
        kind: "numeric_literal",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"\"([^"\\]|\\.)*\""#,
        kind: "string_literal",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"'([^'\\]|\\.)*'"#,
        kind: "string_literal",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"=>",
        kind: "arrow",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\(|\)",
        kind: "parenthesis",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\{",
        kind: "open_brace",
        keep: true,
        modification: StateModification::Push(BLOCK),
    },
    LexerRule {
        pattern: r"[\[\]\.,;:<>=]",
        kind: "punctuation",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"[^\s\w$"'()/\[\]\.,;:<>=]+"#,
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

// Inside a block (e.g. function body), everything is treated as body content
// and discarded by the lexer until the matching closing curly brace is found.
static BLOCK: &[LexerRule] = &[
    LexerRule {
        pattern: r"\}",
        kind: "closing_brace",
        keep: true,
        modification: StateModification::Pop,
    },
    LexerRule {
        pattern: r"[^}]+",
        kind: "content",
        keep: false,
        modification: StateModification::None,
    },
];

pub fn register() -> LanguageConfig<'static> {
    LanguageConfig {
        name: "TypeScript",
        build_lexer: |input| {
            LazyStatefulLexer::new(input, STATEMENTS.to_vec())
                .map(|lexer| Box::new(lexer) as Box<dyn lexer::Lexer>)
        },
        parser: parse_at_anchors(map! {
            AnchorLocation::Exact { token_kind: "keyword", text: "function" } => vec![
                Box::new(keyworded_function_definition) as Parser<LanguageFeature>,
            ]
        }),
    }
}
