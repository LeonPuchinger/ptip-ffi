use crate::{
    config::LanguageConfig,
    features::LanguageFeature,
    map,
    parser::{
        ParserError,
        atoms::{exact, token_kind},
        lexer::{self, LazyStatefulLexer, LexerRule, StateModification},
    },
};

fn keyworded_function_definition(
    lexer: &mut dyn lexer::Lexer,
) -> Result<LanguageFeature, ParserError> {
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

fn statements(lexer: &mut dyn lexer::Lexer) -> Result<Vec<LanguageFeature>, ParserError> {
    let anchors = map! {
        ("keyword", "function") => vec![keyworded_function_definition]
    };
    let mut features = Vec::new();
    'anchor: loop {
        let next = match lexer.peek() {
            Ok(token) => token,
            Err(lexer::LexerError::Eof) => break 'anchor,
            Err(error) => return Err(error.into()),
        };
        if let Some(parsers) = anchors.get(&(next.kind, next.text)) {
            let before_anchor = lexer.snapshot();
            for parser in parsers {
                lexer.restore(before_anchor);
                if let Ok(feature) = parser(lexer) {
                    features.push(feature);
                    // Assert whether the successful parser actually consumed any tokens
                    if lexer.snapshot().input_cursor == before_anchor.input_cursor {
                        return Err(ParserError::Custom(format!(
                            "Parser for anchor {:?} did not consume any tokens",
                            (next.kind, next.text)
                        )));
                    }
                    continue 'anchor;
                }
            }
            lexer.restore(before_anchor);
        }
        // No parser matched or the token is not an anchor.
        // In either case, the lexer needs to be advanced one token.
        match lexer.next() {
            Ok(_) => continue,
            Err(lexer::LexerError::Eof) => break 'anchor,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(features)
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

pub fn register() -> LanguageConfig {
    LanguageConfig {
        name: "TypeScript",
        build_lexer: |input| {
            LazyStatefulLexer::new(input, STATEMENTS.to_vec())
                .map(|lexer| Box::new(lexer) as Box<dyn lexer::Lexer>)
        },
        parser: statements,
    }
}
