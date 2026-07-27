use crate::{
    features::{Function, FunctionParameter, Module, PrimitiveType, Type},
    map,
    parser::{
        Parser, ParserError,
        atoms::{exact, token_kind},
        combinators::{AnchorLocation, AnchorRule, optional, parse_at_anchors},
        lexer::{LazyStatefulLexer, LexerRule, StateModification},
    },
};

fn function_parameter<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionParameter, ParserError> {
    let parameter_name = token_kind("identifier")(lexer)?;
    let required = optional(exact("?"))(lexer)?.is_none();
    exact(":")(lexer)?;
    let parameter_type = optional(token_kind("identifier"))(lexer)?;
    Ok(FunctionParameter {
        name: parameter_name,
        r#type: parameter_type
            .map(|type_name| match type_name.as_str() {
                "number" => Type::Primitive(PrimitiveType::Number),
                "string" => Type::Primitive(PrimitiveType::String),
                "boolean" => Type::Primitive(PrimitiveType::Boolean),
                _ => Type::Composite {
                    name: type_name,
                    path: Vec::new(),
                },
            })
            .unwrap_or(Type::Undefined),
        required,
    })
}

fn function_parameters<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Vec<FunctionParameter>, ParserError> {
    let mut params = Vec::new();
    lexer.push_state("parameters")?;
    loop {
        match optional(Box::new(|lexer| function_parameter(lexer)))(lexer)? {
            Some(param) => params.push(param),
            None => break,
        }
        match optional(exact(","))(lexer)? {
            Some(_) => continue,
            None => break,
        }
    }
    lexer.pop_state()?;
    Ok(params)
}

fn keyworded_function_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Function, ParserError> {
    exact("function")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    exact("(")(lexer)?;
    let parameters = function_parameters(lexer)?;
    exact(")")(lexer)?;
    exact("{")(lexer)?;
    exact("}")(lexer)?;
    Ok(Function {
        name,
        positional_parameters: parameters,
        named_parameters: Vec::new(),
        return_type: Type::Undefined,
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

static FUNCTION_PARAMETERS: &[LexerRule] = &[
    LexerRule {
        pattern: r"[ \t\r\n]+",
        kind: "whitespace",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"[A-Za-z_$][A-Za-z0-9_$]*",
        kind: "identifier",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\?|:",
        kind: "parameter_syntax",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r",",
        kind: "comma",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"[\(\)]",
        kind: "parenthesis",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"[^\s\w$?:,()]+"#,
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

pub fn parse(input: &str) -> Result<Module, ParserError> {
    let mut lexer = LazyStatefulLexer::new(
        input,
        map! {
            "statements" => STATEMENTS.to_vec(),
            "parameters" => FUNCTION_PARAMETERS.to_vec(),
            "block" => BLOCK.to_vec(),
        },
        "statements",
    )?;
    let module = parse_at_anchors(
        Module {
            name: String::new(),
            children: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
        },
        map! {
            AnchorLocation::Exact { token_kind: "keyword", text: "function" } => AnchorRule {
                parsers: vec![
                    Box::new(keyworded_function_definition) as Parser<LazyStatefulLexer<'_>, Function>,
                ],
                reducer: Box::new(|mut module: Module, function: Function| {
                    module.functions.push(function);
                    module
                }),
                _input: std::marker::PhantomData,
            }
        },
    )(&mut lexer)?;
    Ok(module)
}
