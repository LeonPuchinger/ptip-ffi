use crate::{
    config::LanguageConfig,
    features::{FunctionParameter, LanguageFeature, Type},
    map,
    parser::{
        Parser, ParserError,
        atoms::{exact, token_kind},
        combinators::{AnchorLocation, optional, parse_at_anchors},
        lexer::{self, LazyStatefulLexer, Lexer, LexerRule, StateModification},
    },
};

fn function_parameter(lexer: &mut dyn Lexer) -> Result<FunctionParameter, ParserError> {
    let parameter_name = token_kind("identifier")(lexer)?;
    let required = optional(exact("?"))(lexer)?.is_none();
    exact(":")(lexer)?;
    let parameter_type = optional(token_kind("identifier"))(lexer)?;
    Ok(FunctionParameter {
        name: parameter_name,
        r#type: parameter_type.map(|type_name| Type {
            name: type_name,
            path: Vec::new(),
        }),
        required,
    })
}

fn function_parameters(lexer: &mut dyn Lexer) -> Result<Vec<FunctionParameter>, ParserError> {
    let mut params = Vec::new();
    loop {
        match optional(Box::new(function_parameter))(lexer)? {
            Some(param) => params.push(param),
            None => break,
        }
        match optional(exact(","))(lexer)? {
            Some(_) => continue,
            None => break,
        }
    }
    Ok(params)
}

fn keyworded_function_definition(lexer: &mut dyn Lexer) -> Result<LanguageFeature, ParserError> {
    exact("function")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    exact("(")(lexer)?;
    let parameters = function_parameters(lexer)?;
    exact(")")(lexer)?;
    exact("{")(lexer)?;
    exact("}")(lexer)?;
    Ok(LanguageFeature::Function {
        name,
        args: parameters,
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
        modification: StateModification::Push("block"),
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
        parse: |input| {
            let mut lexer = LazyStatefulLexer::new(
                input,
                map! {
                    "statements" => STATEMENTS.to_vec(),
                    "block" => BLOCK.to_vec(),
                },
                "statements",
            )?;
            let features = parse_at_anchors(map! {
                AnchorLocation::Exact { token_kind: "keyword", text: "function" } => vec![
                    Box::new(keyworded_function_definition) as Parser<LanguageFeature>,
                ]
            })(&mut lexer)?;
            Ok(features)
        },
    }
}
