use crate::{
    features::{AnonymousCallable, FunctionDefinition, Module, ModulePath, Type, TypeDefinition, ValueParameter},
    map,
    parser::{
        atoms::{exact, token_kind},
        combinators::{optional, AnchorLocation, AnchorRule, parse_at_anchors},
        lexer::{LazyStatefulLexer, Lexer, LexerError, LexerRule, StateModification},
        Parser, ParserError,
    },
};

fn sanitize_source(input: &str) -> String {
    let mut sanitized = String::new();
    let mut collecting_header = false;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;

    for line in input.lines() {
        let trimmed_start = line.trim_start();
        let indentation = line.len() - trimmed_start.len();

        if trimmed_start.is_empty() {
            sanitized.push('\n');
            continue;
        }

        if !collecting_header {
            if indentation == 0 && (trimmed_start.starts_with("def ") || trimmed_start.starts_with("class ")) {
                collecting_header = true;
                paren_depth = 0;
                bracket_depth = 0;
            } else {
                continue;
            }
        }

        sanitized.push_str(line);
        sanitized.push('\n');

        for character in trimmed_start.chars() {
            match character {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                ':' if paren_depth == 0 && bracket_depth == 0 => {
                    collecting_header = false;
                    break;
                }
                _ => {}
            }
        }
    }

    sanitized
}

fn consume_until<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
    stop_texts: &[&str],
) -> Result<(), ParserError> {
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut brace_depth = 0i32;

    loop {
        let snapshot = lexer.snapshot();
        let token = match lexer.next() {
            Ok(token) => token,
            Err(LexerError::Eof) => return Ok(()),
            Err(error) => return Err(error.into()),
        };

        if paren_depth == 0
            && bracket_depth == 0
            && brace_depth == 0
            && stop_texts.contains(&token.text)
        {
            lexer.restore(&snapshot);
            return Ok(());
        }

        match token.text {
            "(" => paren_depth += 1,
            ")" => paren_depth -= 1,
            "[" => bracket_depth += 1,
            "]" => bracket_depth -= 1,
            "{" => brace_depth += 1,
            "}" => brace_depth -= 1,
            _ => {}
        }
    }
}

fn parse_type_hints<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<(), ParserError> {
    if optional(exact(":"))(lexer)?.is_some() {
        consume_until(lexer, &[",", "=", ")"])?;
    }
    Ok(())
}

fn parse_function_parameters<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<Vec<ValueParameter>, ParserError> {
    let mut parameters = Vec::new();
    exact("(")(lexer)?;
    loop {
        if optional(exact(")"))(lexer)?.is_some() {
            break;
        }

        if optional(exact("/"))(lexer)?.is_some() {
            let _ = optional(exact(","))(lexer)?;
            continue;
        }

        let mut variadic = false;
        if optional(exact("*"))(lexer)?.is_some() {
            variadic = true;
            let has_double_star = optional(exact("*"))(lexer)?.is_some();
            if has_double_star {
                variadic = true;
            }

            let next_token_is_identifier = lexer
                .peek()
                .map(|token| token.kind == "identifier")
                .unwrap_or(false);
            if !next_token_is_identifier {
                let _ = optional(exact(","))(lexer)?;
                continue;
            }
        }

        let name = token_kind("identifier")(lexer)?;
        parse_type_hints(lexer)?;

        let required = !optional(exact("="))(lexer)?.is_some();
        if !required {
            consume_until(lexer, &[",", ")"])?;
        }

        parameters.push(ValueParameter {
            name,
            r#type: Type::Dynamic,
            required,
            variadic,
            nullable: false,
        });

        if optional(exact(","))(lexer)?.is_some() {
            continue;
        }
        exact(")")(lexer)?;
        break;
    }
    Ok(parameters)
}

fn consume_optional_type_parameters<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<(), ParserError> {
    if optional(exact("["))(lexer)?.is_none() {
        return Ok(());
    }
    consume_until(lexer, &["]"])?;
    exact("]")(lexer)?;
    Ok(())
}

fn parse_function_definition<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<FunctionDefinition, ParserError> {
    exact("def")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    consume_optional_type_parameters(lexer)?;
    let positional_parameters = parse_function_parameters(lexer)?;
    if optional(exact("->"))(lexer)?.is_some() {
        consume_until(lexer, &[":"])?;
    }
    exact(":")(lexer)?;

    Ok(FunctionDefinition {
        name,
        callable: AnonymousCallable {
            positional_parameters,
            named_parameters: Vec::new(),
            return_type: Type::Dynamic,
            type_parameters: Vec::new(),
        },
    })
}

fn parse_class_definition<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<TypeDefinition, ParserError> {
    exact("class")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    consume_optional_type_parameters(lexer)?;
    consume_until(lexer, &[":"])?;
    exact(":")(lexer)?;

    Ok(TypeDefinition {
        name,
        properties: Vec::new(),
        default_constructor: None,
        named_constructors: Vec::new(),
        methods: Vec::new(),
        static_methods: Vec::new(),
        type_parameters: Vec::new(),
        implements: Vec::new(),
    })
}

enum ParsedFeature {
    Function(FunctionDefinition),
    Type(TypeDefinition),
}

fn reduce_parsed_feature(mut module: Module, feature: ParsedFeature) -> Module {
    match feature {
        ParsedFeature::Function(function) => module.functions.push(function),
        ParsedFeature::Type(definition) => module.types.push(definition),
    }
    module
}

static STATEMENTS: &[LexerRule] = &[
    LexerRule {
        pattern: r"[ \t\r\n]+",
        kind: "whitespace",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"#[^\n]*",
        kind: "comment",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\b(def|class|pass|return|async|lambda|from|import|as|with|if|elif|else|try|except|finally|for|while|match|case)\b",
        kind: "keyword",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"[A-Za-z_][A-Za-z0-9_]*",
        kind: "identifier",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"->|\*\*|\.\.\.|\(|\)|\[|\]|\{|\}|,|:|\.|=|\*|\|",
        kind: "punctuation",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"'([^'\\]|\\.)*'|\"([^\"\\]|\\.)*\""#,
        kind: "string_literal",
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
        pattern: r#"[^\s\w#'"\(\)\[\]\{\},:\.=\*\|]+"#,
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

pub fn parse(input: &str) -> Result<Module, ParserError> {
    let sanitized_input = sanitize_source(input);
    let mut lexer = LazyStatefulLexer::new(
        sanitized_input.as_str(),
        map! {
            "statements" => STATEMENTS.to_vec(),
        },
        "statements",
    )?;

    parse_at_anchors(
        Module {
            path: ModulePath::empty(),
            functions: Vec::new(),
            types: Vec::new(),
        },
        map! {
            AnchorLocation::Exact { token_kind: "keyword", text: "def" } => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| parse_function_definition(lexer).map(ParsedFeature::Function)) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Exact { token_kind: "keyword", text: "class" } => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| parse_class_definition(lexer).map(ParsedFeature::Type)) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
        },
    )(&mut lexer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_or_panic(input: &str) -> Module {
        parse(input).unwrap_or_else(|error| panic!("parse failed: {:?}", error))
    }

    #[test]
    fn parses_top_level_functions_and_classes_with_dynamic_types() {
        let module = parse_or_panic(
            r#"
def add(left: int, right: list[str]) -> int:
    return left + right

class Box[T](Generic[T]):
    pass
"#,
        );

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.types.len(), 1);
        assert!(matches!(module.functions[0].callable.positional_parameters[0].r#type, Type::Dynamic));
        assert!(matches!(module.functions[0].callable.positional_parameters[1].r#type, Type::Dynamic));
        assert!(matches!(module.functions[0].callable.return_type, Type::Dynamic));
        assert_eq!(module.types[0].name, "Box");
    }
}