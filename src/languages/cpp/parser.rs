use std::marker::PhantomData;

use crate::{
    features::{
        AnonymousCallable, FunctionDefinition, Method, Module, ModulePath, PrimitiveType, Type,
        TypeDefinition, TypeParameter, TypePath, ValueParameter,
    },
    map,
    parser::{
        Parser, ParserError,
        atoms::{exact, token_kind},
        combinators::{AnchorLocation, AnchorRule, optional, parse_at_anchors},
        lexer::{LazyStatefulLexer, Lexer, LexerError, LexerRule, StateModification},
    },
};

#[derive(Clone)]
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

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut last_was_space = false;

    let mut chars = input.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '<' => {
                if paren_depth == 0 && bracket_depth == 0 {
                    angle_depth += 1;
                }
            }
            '>' => {
                if paren_depth == 0 && bracket_depth == 0 && angle_depth > 0 {
                    angle_depth -= 1;
                }
            }
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            '[' => bracket_depth += 1,
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            ',' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                segments.push(input[start..index].trim());
                start = index + ch.len_utf8();
                last_was_space = false;
                continue;
            }
            _ => {}
        }
        last_was_space = ch.is_whitespace() && last_was_space;
    }

    if start <= input.len() {
        let tail = input[start..].trim();
        if !tail.is_empty() {
            segments.push(tail);
        }
    }
    segments
}

fn split_top_level_semicolons(input: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;

    let mut chars = input.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '{' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => brace_depth += 1,
            '}' if angle_depth == 0 && paren_depth == 0 && bracket_depth == 0 => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            }
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            '[' => bracket_depth += 1,
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '<' => {
                if paren_depth == 0 && bracket_depth == 0 {
                    angle_depth += 1;
                }
            }
            '>' => {
                if angle_depth > 0 && paren_depth == 0 && bracket_depth == 0 {
                    angle_depth -= 1;
                }
            }
            ';' if brace_depth == 0 && paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                let segment = input[start..index].trim();
                if !segment.is_empty() {
                    chunks.push(segment.to_string());
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        chunks.push(tail.to_string());
    }
    chunks
}

fn parse_type_parameters(raw: &str) -> Vec<TypeParameter> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let params = if let (Some(open), Some(close)) = (trimmed.find('<'), trimmed.rfind('>')) {
        &trimmed[open + 1..close]
    } else {
        trimmed
    };

    split_top_level_commas(params)
        .into_iter()
        .filter_map(|part| {
            let value = part
                .replace("typename ", "")
                .replace("class ", "")
                .split('=')
                .next()
                .unwrap_or(part)
                .trim()
                .to_string();
            if value.is_empty() {
                None
            } else {
                Some(TypeParameter {
                    name: value,
                    default: None,
                })
            }
        })
        .collect()
}

fn parse_type_name(type_name: &str) -> Type {
    let cleaned = type_name
        .replace("const ", "")
        .replace("volatile ", "")
        .replace("&", "")
        .replace("*", "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return Type::Dynamic;
    }
    match cleaned.as_str() {
        "int" | "long" | "short" => Type::Primitive(PrimitiveType::Number),
        "double" | "float" => Type::Primitive(PrimitiveType::Number),
        "bool" => Type::Primitive(PrimitiveType::Boolean),
        "std::string" | "string" => Type::Primitive(PrimitiveType::String),
        "void" => Type::Dynamic,
        _ => {
            let type_arguments = if let Some(open) = cleaned.find('<')
                && let Some(close) = cleaned.rfind('>')
            {
                let inner = &cleaned[open + 1..close];
                split_top_level_commas(inner)
                    .into_iter()
                    .map(parse_type_name)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };

            let base = if cleaned.contains("::") {
                let mut segments = cleaned.split("::").filter(|segment| !segment.is_empty()).collect::<Vec<_>>();
                let name = segments.pop().unwrap_or(&cleaned).to_string();
                let module_path = ModulePath::new(segments.iter().copied().collect::<Vec<_>>());
                TypePath { module_path, name, type_arguments: Vec::new() }
            } else {
                TypePath {
                    module_path: ModulePath::empty(),
                    name: cleaned.clone(),
                    type_arguments: Vec::new(),
                }
            };

            let mut result = Type::Composite(base);
            if !type_arguments.is_empty() {
                if let Type::Composite(path) = &mut result {
                    path.type_arguments = type_arguments;
                }
            }
            result
        }
    }
}

fn parse_parameter_text(parameter: &str) -> Option<ValueParameter> {
    let cleaned = parameter.trim();
    if cleaned.is_empty() || cleaned == "void" || cleaned == "..." {
        return None;
    }
    let trimmed = cleaned.strip_suffix("const").unwrap_or(cleaned).trim();
    let identifier_start = trimmed.rfind(char::is_whitespace)
        .map(|index| trimmed[index + 1..].trim_start().to_string())
        .unwrap_or_else(|| trimmed.to_string());
    let name = if identifier_start.is_empty() {
        return None;
    } else {
        let without_qualifiers = identifier_start
            .trim_end_matches(&['&', '*'][..])
            .to_string();
        without_qualifiers
    };
    let type_part = trimmed.strip_suffix(&name).unwrap_or(trimmed).trim();
    let name = name.trim_end_matches(['&', '*']);
    let type_part = type_part.trim();
    Some(ValueParameter {
        name: name.to_string(),
        r#type: parse_type_name(type_part),
        required: true,
        variadic: false,
        nullable: false,
    })
}

fn parse_callable(signature: &str, type_parameters: Vec<TypeParameter>) -> AnonymousCallable {
    let open = signature.find('(').unwrap_or(signature.len());
    let close = signature.rfind(')').unwrap_or(signature.len());
    let before_paren = signature[..open].trim();
    let params = &signature[open + 1..close];
    let name_end = before_paren.rfind(char::is_whitespace).unwrap_or(before_paren.len());
    let mut return_type = before_paren[..name_end].trim();
    let name = before_paren[name_end..].trim();
    if name.is_empty() {
        return AnonymousCallable {
            positional_parameters: Vec::new(),
            named_parameters: Vec::new(),
            return_type: Type::Dynamic,
            type_parameters,
        };
    }
    if return_type.is_empty() {
        return_type = "void";
    }
    let positional = split_top_level_commas(params)
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .filter_map(parse_parameter_text)
        .collect::<Vec<_>>();
    AnonymousCallable {
        positional_parameters: positional,
        named_parameters: Vec::new(),
        return_type: parse_type_name(return_type),
        type_parameters,
    }
}

fn parse_function_from_text(signature: &str) -> Result<FunctionDefinition, ParserError> {
    let signature = signature.trim();
    if !signature.contains('(') || !signature.contains(')') {
        return Err(ParserError::Custom(format!("Not a function signature: {signature}")));
    }
    let open = signature.find('(').unwrap();
    let before = signature[..open].trim();
    let last_space = before.rfind(char::is_whitespace).unwrap_or(before.len());
    let name = before[last_space..].trim();
    let return_type = before[..last_space].trim();
    if name.is_empty() || return_type.is_empty() {
        return Err(ParserError::Custom(format!("Malformed function signature: {signature}")));
    }
    let callable = parse_callable(signature, Vec::new());
    Ok(FunctionDefinition {
        name: name.to_string(),
        callable,
    })
}

fn parse_type_body(body: &str, type_name: &str) -> TypeDefinition {
    let mut properties = Vec::new();
    let mut methods = Vec::new();
    let mut default_constructor: Option<AnonymousCallable> = None;
    let mut named_constructors = Vec::new();
    let static_methods = Vec::new();

    for member in split_top_level_semicolons(body) {
        let member = member.trim();
        if member.is_empty() || member.starts_with("public:") || member.starts_with("private:") || member.starts_with("protected:") {
            continue;
        }
        let member_upper = member.replace("const", "").trim().to_string();
        if member_upper.contains('(') && member_upper.contains(')') {
            let open = member_upper.find('(').unwrap();
            let return_part = member_upper[..open].trim();
            let last_space = return_part.rfind(char::is_whitespace).unwrap_or(return_part.len());
            let method_name = return_part[last_space..].trim();
            let method_return = return_part[..last_space].trim();
            let callable = parse_callable(&member_upper, Vec::new());
            let method = Method {
                name: method_name.to_string(),
                r#static: false,
                callable: callable.clone(),
            };
            if method_name == type_name {
                default_constructor = Some(callable.clone());
            } else if method_return == type_name {
                named_constructors.push(FunctionDefinition {
                    name: method_name.to_string(),
                    callable: callable.clone(),
                });
            } else {
                methods.push(method);
            }
            continue;
        }
        let mut parts = member.split_whitespace();
        let type_part = parts.next();
        let name_part = parts.next();
        if let (Some(type_name_text), Some(property_name)) = (type_part, name_part) {
            properties.push((property_name.to_string(), parse_type_name(type_name_text)));
        }
    }

    TypeDefinition {
        name: type_name.to_string(),
        properties,
        default_constructor,
        named_constructors,
        methods,
        static_methods,
        type_parameters: Vec::new(),
        implements: Vec::new(),
    }
}

fn parse_struct_or_class<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
    kind: &str,
) -> Result<TypeDefinition, ParserError> {
    exact(kind)(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    let type_parameters = if optional(exact("<"))(lexer)?.is_some() {
        let mut params = Vec::new();
        loop {
            let token = lexer.next()?;
            if token.text == ">" {
                break;
            }
            params.push(token.text.to_string());
        }
        let param_string = params.join(" ");
        parse_type_parameters(&param_string)
    } else {
        Vec::new()
    };
    exact("{")(lexer)?;
    let mut body_tokens = Vec::new();
    let mut depth = 1usize;
    while depth > 0 {
        let token = lexer.next()?;
        match token.text {
            "{" => {
                depth += 1;
                if depth > 1 {
                    body_tokens.push(token.text.to_string());
                }
            }
            "}" => {
                depth -= 1;
                if depth > 0 {
                    body_tokens.push(token.text.to_string());
                }
            }
            _ => body_tokens.push(token.text.to_string()),
        }
    }
    let mut definition = parse_type_body(&body_tokens.join(" "), &name);
    definition.type_parameters = type_parameters;
    if optional(exact(";"))(lexer)?.is_some() {
        // no-op, declarations can end with a trailing semicolon.
    }
    Ok(definition)
}

fn parse_function_signature_from_lexer<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<String, ParserError> {
    let mut tokens = Vec::new();
    loop {
        match lexer.peek() {
            Ok(token) => {
                if token.text == ";" || token.text == "{" || token.text == "}" {
                    break;
                }
                let next = lexer.next()?;
                tokens.push(next.text.to_string());
                if next.text == "(" {
                    let mut depth = 1usize;
                    while depth > 0 {
                        match lexer.peek() {
                            Ok(token) => {
                                let next_token = lexer.next()?;
                                tokens.push(next_token.text.to_string());
                                match next_token.text {
                                    "(" => depth += 1,
                                    ")" => {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Err(LexerError::Eof) => break,
                            Err(err) => return Err(err.into()),
                        }
                    }
                    break;
                }
            }
            Err(LexerError::Eof) => break,
            Err(err) => return Err(err.into()),
        }
    }

    let text = tokens.join(" ");
    if text.contains('(') && text.contains(')') {
        Ok(text)
    } else {
        Err(ParserError::Custom("not a function-like declaration".to_string()))
    }
}

fn parse_template_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<ParsedFeature, ParserError> {
    exact("template")(lexer)?;
    let mut template_tokens = Vec::new();
    if optional(exact("<"))(lexer)?.is_some() {
        loop {
            let token = lexer.next()?;
            if token.text == ">" {
                break;
            }
            template_tokens.push(token.text.to_string());
        }
    }
    let type_parameters = parse_type_parameters(&template_tokens.join(" "));

    let next_kind = match lexer.peek() {
        Ok(token) => token.text.to_string(),
        Err(_) => String::new(),
    };

    if next_kind == "struct" {
        let definition = parse_struct_or_class(lexer, "struct")?;
        let mut definition = definition;
        definition.type_parameters = type_parameters;
        return Ok(ParsedFeature::Type(definition));
    }
    if next_kind == "class" {
        let definition = parse_struct_or_class(lexer, "class")?;
        let mut definition = definition;
        definition.type_parameters = type_parameters;
        return Ok(ParsedFeature::Type(definition));
    }

    let function_text = parse_function_signature_from_lexer(lexer)?;
    let function = parse_function_from_text(&function_text)?;
    let mut function = function;
    function.callable.type_parameters = type_parameters;
    Ok(ParsedFeature::Function(function))
}

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
        pattern: r"\b(template|struct|class|public|private|protected|typename|class|const|volatile)\b",
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
        pattern: r"[0-9]+(?:\.[0-9]+)?",
        kind: "numeric_literal",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\{|\}|\(|\)|;|:|,|<|>|\.|::|\*|&|=|\+|-|/|%|\[|\]",
        kind: "punctuation",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"[^\sA-Za-z0-9_{}();:,.<>\[\]\*&=+/\-%]+",
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

pub fn parse(input: &str) -> Result<Module, ParserError> {
    let mut lexer = LazyStatefulLexer::new(input, map! { "statements" => STATEMENTS.to_vec() }, "statements")?;

    parse_at_anchors(
        Module {
            path: ModulePath { segments: Vec::new() },
            functions: Vec::new(),
            types: Vec::new(),
        },
        map! {
            AnchorLocation::Exact { token_kind: "keyword", text: "template" } => AnchorRule {
                parsers: vec![Box::new(parse_template_definition) as Parser<LazyStatefulLexer<'_>, ParsedFeature>],
                reducer: Box::new(reduce_parsed_feature),
                _input: PhantomData,
            },
            AnchorLocation::Exact { token_kind: "keyword", text: "struct" } => AnchorRule {
                parsers: vec![Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                    parse_struct_or_class(lexer, "struct").map(ParsedFeature::Type)
                }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>],
                reducer: Box::new(reduce_parsed_feature),
                _input: PhantomData,
            },
            AnchorLocation::Exact { token_kind: "keyword", text: "class" } => AnchorRule {
                parsers: vec![Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                    parse_struct_or_class(lexer, "class").map(ParsedFeature::Type)
                }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>],
                reducer: Box::new(reduce_parsed_feature),
                _input: PhantomData,
            },
            AnchorLocation::Kind("identifier") => AnchorRule {
                parsers: vec![Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                    let snapshot = lexer.snapshot();
                    let result = parse_function_from_text(&{
                        let mut tokens = Vec::new();
                        loop {
                            match lexer.peek() {
                                Ok(token) => {
                                    if token.text == ";" || token.text == "{" || token.text == "}" {
                                        break;
                                    }
                                    let next = lexer.next()?;
                                    tokens.push(next.text.to_string());
                                    if next.text == "(" {
                                        let mut depth = 1usize;
                                        loop {
                                            if let Ok(token) = lexer.peek() {
                                                let next_token = lexer.next()?;
                                                tokens.push(next_token.text.to_string());
                                                match next_token.text {
                                                    "(" => depth += 1,
                                                    ")" => {
                                                        depth -= 1;
                                                        if depth == 0 {
                                                            break;
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                            } else {
                                                break;
                                            }
                                            if depth == 0 {
                                                break;
                                            }
                                        }
                                        break;
                                    }
                                }
                                Err(LexerError::Eof) => break,
                                Err(err) => return Err(err.into()),
                            }
                        }
                        let text = tokens.join(" ");
                        if text.contains('(') && text.contains(')') {
                            text
                        } else {
                            return Err(ParserError::Custom("not a function-like declaration".to_string()));
                        }
                    });
                    if result.is_ok() {
                        return result.map(ParsedFeature::Function);
                    }
                    lexer.restore(&snapshot);
                    Err(ParserError::Custom("not a function-like declaration".to_string()))
                }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>],
                reducer: Box::new(reduce_parsed_feature),
                _input: PhantomData,
            },
        },
    )(&mut lexer)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_named_function_definition() {
        let module = parse("int add(int x, int y) { return x + y; }").unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "add");
    }

    #[test]
    fn parses_struct_definition() {
        let module = parse("struct Point { int x; int y; };\n").unwrap();
        assert_eq!(module.types.len(), 1);
        assert_eq!(module.types[0].name, "Point");
    }

    #[test]
    fn parses_class_with_method() {
        let module = parse(
            "class Widget { public: Widget(int value); int get_value() const; };\n",
        )
        .unwrap();
        assert_eq!(module.types.len(), 1);
        assert_eq!(module.types[0].name, "Widget");
        assert_eq!(module.types[0].methods.len(), 1);
    }

    #[test]
    fn parses_generic_function_template() {
        let module = parse("template <typename T> T identity(T value) { return value; }").unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "identity");
        assert_eq!(module.functions[0].callable.type_parameters.len(), 1);
    }

    #[test]
    fn parses_generic_struct_template() {
        let module = parse("template <typename T> struct Box { T value; };\n").unwrap();
        assert_eq!(module.types.len(), 1);
        assert_eq!(module.types[0].name, "Box");
        assert_eq!(module.types[0].type_parameters.len(), 1);
    }
}
