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

fn primitive_or_composite(name: String) -> Type {
    match name.as_str() {
        "number" => Type::Primitive(PrimitiveType::Number),
        "string" => Type::Primitive(PrimitiveType::String),
        "boolean" => Type::Primitive(PrimitiveType::Boolean),
        _ => Type::Composite(TypePath {
            module_path: ModulePath::empty(),
            name,
        }),
    }
}

fn parse_type<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<Type, ParserError> {
    let mut base = if optional(exact("["))(lexer)?.is_some() {
        // Distinguish tuple types like [number, string] from array suffixes handled later.
        let mut elements = Vec::new();
        if optional(exact("]"))(lexer)?.is_none() {
            loop {
                elements.push(parse_type(lexer)?);
                if optional(exact(","))(lexer)?.is_some() {
                    continue;
                }
                exact("]")(lexer)?;
                break;
            }
        }
        Type::Tuple(elements)
    } else if let Some(name) = optional(token_kind("identifier"))(lexer)? {
        let base = primitive_or_composite(name);

        // Parse and ignore generic type arguments. The current feature model stores
        // only the outer type path.
        if optional(exact("<"))(lexer)?.is_some() {
            let mut depth = 1usize;
            while depth > 0 {
                match lexer.next() {
                    Ok(token) if token.text == "<" => depth += 1,
                    Ok(token) if token.text == ">" => depth -= 1,
                    Ok(_) => {}
                    Err(LexerError::Eof) => return Err(ParserError::UnexpectedEof),
                    Err(e) => return Err(e.into()),
                }
            }
        }
        base
    } else {
        Type::Dynamic
    };

    while optional(exact("["))(lexer)?.is_some() {
        exact("]")(lexer)?;
        base = Type::Array(Box::new(base));
    }

    Ok(base)
}

fn parse_type_parameters<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Vec<TypeParameter>, ParserError> {
    let mut result = Vec::new();
    if optional(exact("<"))(lexer)?.is_none() {
        return Ok(result);
    }

    loop {
        let name = token_kind("identifier")(lexer)?;

        // Optional "extends" constraint is consumed and ignored for now.
        if optional(exact("extends"))(lexer)?.is_some() {
            let _ = parse_type(lexer)?;
        }

        let default = if optional(exact("="))(lexer)?.is_some() {
            Some(parse_type(lexer)?)
        } else {
            None
        };

        result.push(TypeParameter { name, default });

        if optional(exact(","))(lexer)?.is_some() {
            continue;
        }
        exact(">")(lexer)?;
        break;
    }

    Ok(result)
}

fn function_parameter<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<ValueParameter, ParserError> {
    let variadic = optional(exact("..."))(lexer)?.is_some();
    let parameter_name = token_kind("identifier")(lexer)?;
    let required = optional(exact("?"))(lexer)?.is_none();
    let parameter_type = if optional(exact(":"))(lexer)?.is_some() {
        parse_type(lexer)?
    } else {
        Type::Dynamic
    };
    Ok(ValueParameter {
        name: parameter_name,
        r#type: parameter_type,
        required,
        variadic,
        nullable: false,
    })
}

fn function_parameters<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Vec<ValueParameter>, ParserError> {
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

fn parse_callable_signature<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<AnonymousCallable, ParserError> {
    let type_parameters = parse_type_parameters(lexer)?;
    exact("(")(lexer)?;
    let parameters = function_parameters(lexer)?;
    exact(")")(lexer)?;
    let return_type = if optional(exact(":"))(lexer)?.is_some() {
        parse_type(lexer)?
    } else {
        Type::Dynamic
    };
    Ok(AnonymousCallable {
        positional_parameters: parameters,
        named_parameters: Vec::new(),
        return_type,
        type_parameters,
    })
}

fn consume_statement_terminator<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<(), ParserError> {
    match exact(";")(lexer) {
        Ok(_) => Ok(()),
        Err(ParserError::UnexpectedToken { .. }) | Err(ParserError::UnexpectedEof) => Ok(()),
        Err(e) => Err(e),
    }
}

fn consume_balanced_block<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<(), ParserError> {
    let mut depth = 1usize;
    while depth > 0 {
        match lexer.next() {
            Ok(token) if token.text == "{" => depth += 1,
            Ok(token) if token.text == "}" => depth -= 1,
            Ok(_) => {}
            Err(LexerError::Eof) => return Err(ParserError::UnexpectedEof),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn consume_function_body_or_terminator<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<(), ParserError> {
    if optional(exact("{"))(lexer)?.is_some() {
        consume_balanced_block(lexer)?;
        return Ok(());
    }
    consume_statement_terminator(lexer)
}

fn parse_type_path<'input>(lexer: &mut LazyStatefulLexer<'input>) -> Result<TypePath, ParserError> {
    let name = token_kind("identifier")(lexer)?;

    // Parse and ignore generic arguments for implements/type references.
    if optional(exact("<"))(lexer)?.is_some() {
        let mut depth = 1usize;
        while depth > 0 {
            match lexer.next() {
                Ok(token) if token.text == "<" => depth += 1,
                Ok(token) if token.text == ">" => depth -= 1,
                Ok(_) => {}
                Err(LexerError::Eof) => return Err(ParserError::UnexpectedEof),
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(TypePath {
        module_path: ModulePath::empty(),
        name,
    })
}

fn parse_implements<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Vec<TypePath>, ParserError> {
    let mut interfaces = Vec::new();
    if optional(exact("implements"))(lexer)?.is_none() {
        return Ok(interfaces);
    }
    loop {
        interfaces.push(parse_type_path(lexer)?);
        if optional(exact(","))(lexer)?.is_some() {
            continue;
        }
        break;
    }
    Ok(interfaces)
}

fn parse_constructor_member<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<AnonymousCallable, ParserError> {
    exact("constructor")(lexer)?;
    exact("(")(lexer)?;
    let positional_parameters = function_parameters(lexer)?;
    exact(")")(lexer)?;
    if optional(exact("{"))(lexer)?.is_some() {
        consume_balanced_block(lexer)?;
    } else {
        consume_statement_terminator(lexer)?;
    }
    Ok(AnonymousCallable {
        positional_parameters,
        named_parameters: Vec::new(),
        return_type: Type::Dynamic,
        type_parameters: Vec::new(),
    })
}

fn parse_method_member<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Method, ParserError> {
    let is_static = optional(exact("static"))(lexer)?.is_some();
    let name = token_kind("identifier")(lexer)?;
    let callable = parse_callable_signature(lexer)?;

    if optional(exact("{"))(lexer)?.is_some() {
        consume_balanced_block(lexer)?;
    } else {
        consume_statement_terminator(lexer)?;
    }

    Ok(Method {
        name,
        r#static: is_static,
        callable,
    })
}

fn parse_property_member<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<(String, Type), ParserError> {
    let name = token_kind("identifier")(lexer)?;
    let _required = optional(exact("?"))(lexer)?.is_none();
    exact(":")(lexer)?;
    let member_type = parse_type(lexer)?;
    let _ = optional(exact(";"))(lexer)?;
    Ok((name, member_type))
}

fn parse_class_body<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
    type_name: &str,
) -> Result<TypeDefinition, ParserError> {
    lexer.push_state("class_open")?;
    exact("{")(lexer)?;

    let mut definition = TypeDefinition {
        name: type_name.to_string(),
        properties: Vec::new(),
        default_constructor: None,
        named_constructors: Vec::new(),
        methods: Vec::new(),
        static_methods: Vec::new(),
        type_parameters: Vec::new(),
        implements: Vec::new(),
    };

    loop {
        if optional(exact("}"))(lexer)?.is_some() {
            break;
        }

        if let Some(constructor) = optional(Box::new(parse_constructor_member))(lexer)? {
            definition.default_constructor = Some(constructor);
            continue;
        }

        if let Some(method) = optional(Box::new(parse_method_member))(lexer)? {
            if method.r#static {
                if let Type::Composite(type_path) = &method.callable.return_type
                    && type_path.name == definition.name
                {
                    definition.named_constructors.push(FunctionDefinition {
                        name: method.name.clone(),
                        callable: method.callable.clone(),
                    });
                }
                definition.static_methods.push(method);
            } else {
                definition.methods.push(method);
            }
            continue;
        }

        if let Some(property) = optional(Box::new(parse_property_member))(lexer)? {
            definition.properties.push(property);
            continue;
        }

        // Ensure progress even for unsupported class members.
        match lexer.next() {
            Ok(_) => continue,
            Err(LexerError::Eof) => return Err(ParserError::UnexpectedEof),
            Err(e) => return Err(e.into()),
        }
    }

    lexer.pop_state()?;
    Ok(definition)
}

fn keyworded_class_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<TypeDefinition, ParserError> {
    exact("class")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    let type_parameters = parse_type_parameters(lexer)?;
    let implements = parse_implements(lexer)?;
    let mut definition = parse_class_body(lexer, &name)?;
    definition.type_parameters = type_parameters;
    definition.implements = implements;
    Ok(definition)
}

fn parse_type_member_method<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<Method, ParserError> {
    let name = token_kind("identifier")(lexer)?;
    let callable = parse_callable_signature(lexer)?;
    let _ = optional(exact(";"))(lexer)?;
    Ok(Method {
        name,
        r#static: false,
        callable,
    })
}

fn parse_type_alias_object_body<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
    type_name: &str,
) -> Result<TypeDefinition, ParserError> {
    lexer.push_state("type_object_open")?;
    exact("{")(lexer)?;

    let mut definition = TypeDefinition {
        name: type_name.to_string(),
        properties: Vec::new(),
        default_constructor: None,
        named_constructors: Vec::new(),
        methods: Vec::new(),
        static_methods: Vec::new(),
        type_parameters: Vec::new(),
        implements: Vec::new(),
    };

    loop {
        if optional(exact("}"))(lexer)?.is_some() {
            break;
        }

        if let Some(method) = optional(Box::new(parse_type_member_method))(lexer)? {
            definition.methods.push(method);
            continue;
        }

        if let Some(property) = optional(Box::new(parse_property_member))(lexer)? {
            definition.properties.push(property);
            continue;
        }

        match lexer.next() {
            Ok(_) => continue,
            Err(LexerError::Eof) => return Err(ParserError::UnexpectedEof),
            Err(e) => return Err(e.into()),
        }
    }

    lexer.pop_state()?;
    Ok(definition)
}

fn type_alias_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<TypeDefinition, ParserError> {
    exact("type")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    let type_parameters = parse_type_parameters(lexer)?;
    exact("=")(lexer)?;

    let open_brace_snapshot = lexer.snapshot();
    let mut definition = if optional(exact("{"))(lexer)?.is_some() {
        lexer.restore(&open_brace_snapshot);
        parse_type_alias_object_body(lexer, &name)?
    } else {
        // Non-object aliases are intentionally represented as empty structural
        // type definitions for now because signatures are the primary goal.
        let _ = parse_type(lexer)?;
        consume_statement_terminator(lexer)?;
        TypeDefinition {
            name: name.clone(),
            properties: Vec::new(),
            default_constructor: None,
            named_constructors: Vec::new(),
            methods: Vec::new(),
            static_methods: Vec::new(),
            type_parameters: Vec::new(),
            implements: Vec::new(),
        }
    };
    definition.type_parameters = type_parameters;
    Ok(definition)
}

fn keyworded_function_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionDefinition, ParserError> {
    exact("function")(lexer)?;
    let name = token_kind("identifier")(lexer)?;
    let callable = parse_callable_signature(lexer)?;
    consume_function_body_or_terminator(lexer)?;
    Ok(FunctionDefinition {
        name,
        callable,
    })
}

fn default_keyworded_anonymous_function<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionDefinition, ParserError> {
    exact("default")(lexer)?;
    exact("function")(lexer)?;
    let callable = parse_callable_signature(lexer)?;
    consume_function_body_or_terminator(lexer)?;
    Ok(FunctionDefinition {
        name: "default".to_string(),
        callable,
    })
}

fn function_expression_rhs<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<AnonymousCallable, ParserError> {
    exact("function")(lexer)?;
    parse_callable_signature(lexer)
}

fn arrow_function_rhs<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<AnonymousCallable, ParserError> {
    let type_parameters = parse_type_parameters(lexer)?;
    exact("(")(lexer)?;
    let positional_parameters = function_parameters(lexer)?;
    exact(")")(lexer)?;
    let return_type = if optional(exact(":"))(lexer)?.is_some() {
        parse_type(lexer)?
    } else {
        Type::Dynamic
    };
    exact("=>")(lexer)?;

    if optional(exact("{"))(lexer)?.is_some() {
        consume_balanced_block(lexer)?;
    } else {
        // Consume expression-bodied arrows until statement end.
        loop {
            if optional(exact(";"))(lexer)?.is_some() {
                break;
            }
            match lexer.next() {
                Ok(_) => continue,
                Err(LexerError::Eof) => break,
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(AnonymousCallable {
        positional_parameters,
        named_parameters: Vec::new(),
        return_type,
        type_parameters,
    })
}

fn bound_function_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionDefinition, ParserError> {
    if optional(exact("const"))(lexer)?.is_none()
        && optional(exact("let"))(lexer)?.is_none()
        && optional(exact("var"))(lexer)?.is_none()
    {
        return Err(ParserError::UnexpectedToken {
            expected: "const|let|var".to_string(),
            found: "other".to_string(),
        });
    }

    let name = token_kind("identifier")(lexer)?;
    exact("=")(lexer)?;

    let callable = if let Some(callable) = optional(Box::new(function_expression_rhs))(lexer)? {
        consume_function_body_or_terminator(lexer)?;
        callable
    } else {
        arrow_function_rhs(lexer)?
    };

    consume_statement_terminator(lexer)?;
    Ok(FunctionDefinition { name, callable })
}

fn exported_keyworded_function_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionDefinition, ParserError> {
    exact("export")(lexer)?;
    keyworded_function_definition(lexer)
}

fn exported_default_keyworded_anonymous_function<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionDefinition, ParserError> {
    exact("export")(lexer)?;
    default_keyworded_anonymous_function(lexer)
}

fn exported_bound_function_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<FunctionDefinition, ParserError> {
    exact("export")(lexer)?;
    bound_function_definition(lexer)
}

fn exported_class_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<TypeDefinition, ParserError> {
    exact("export")(lexer)?;
    keyworded_class_definition(lexer)
}

fn exported_type_alias_definition<'input>(
    lexer: &mut LazyStatefulLexer<'input>,
) -> Result<TypeDefinition, ParserError> {
    exact("export")(lexer)?;
    type_alias_definition(lexer)
}

enum ParsedFeature {
    Function(FunctionDefinition),
    Type(TypeDefinition),
}

fn reduce_parsed_feature(mut module: Module, feature: ParsedFeature) -> Module {
    match feature {
        ParsedFeature::Function(function) => module.functions.push(function),
        ParsedFeature::Type(ty) => module.types.push(ty),
    }
    module
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
        pattern: r"\b(export|default|function|class|type|const|let|var|static|implements|constructor|extends)\b",
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
        pattern: r"\.\.\.",
        kind: "spread",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\(|\)|\[|\]|\.|,|;|:|\?|<|>|=",
        kind: "punctuation",
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
        pattern: r"\}",
        kind: "closing_brace",
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
        pattern: r"\.\.\.",
        kind: "spread",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\?|:|\[|\]|<|>|=",
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
        pattern: r#"[^\s\w$?:,()\[\]<>.=]+"#,
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

// Inside a block (e.g. function body), everything is treated as body content
// and discarded by the lexer until the matching closing curly brace is found.
static BLOCK: &[LexerRule] = &[
    LexerRule {
        pattern: r"\{",
        kind: "open_brace",
        keep: true,
        modification: StateModification::Push("block"),
    },
    LexerRule {
        pattern: r"\}",
        kind: "closing_brace",
        keep: true,
        modification: StateModification::Pop,
    },
    LexerRule {
        pattern: r"[^{}]+",
        kind: "content",
        keep: false,
        modification: StateModification::None,
    },
];

static CLASS_OPEN: &[LexerRule] = &[
    LexerRule {
        pattern: r"[ \t\r\n]+",
        kind: "whitespace",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\{",
        kind: "open_brace",
        keep: true,
        modification: StateModification::Push("class_body"),
    },
    LexerRule {
        pattern: r#"[^\s\{]+"#,
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

static CLASS_BODY: &[LexerRule] = &[
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
        pattern: r"\}",
        kind: "closing_brace",
        keep: true,
        modification: StateModification::Pop,
    },
    LexerRule {
        pattern: r"\{",
        kind: "open_brace",
        keep: true,
        modification: StateModification::Push("block"),
    },
    LexerRule {
        pattern: r"\b(static|constructor|extends)\b",
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
        pattern: r"\.\.\.",
        kind: "spread",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\(|\)|\[|\]|\.|,|;|:|\?|<|>|=",
        kind: "punctuation",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"[^\s\w$\{\}\(\)\[\]\.,;:?<>=]+"#,
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

static TYPE_OBJECT_OPEN: &[LexerRule] = &[
    LexerRule {
        pattern: r"[ \t\r\n]+",
        kind: "whitespace",
        keep: false,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\{",
        kind: "open_brace",
        keep: true,
        modification: StateModification::Push("type_object_body"),
    },
    LexerRule {
        pattern: r#"[^\s\{]+"#,
        kind: "irrelevant",
        keep: false,
        modification: StateModification::None,
    },
];

static TYPE_OBJECT_BODY: &[LexerRule] = &[
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
        pattern: r"\}",
        kind: "closing_brace",
        keep: true,
        modification: StateModification::Pop,
    },
    LexerRule {
        pattern: r"[A-Za-z_$][A-Za-z0-9_$]*",
        kind: "identifier",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\.\.\.",
        kind: "spread",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r"\(|\)|\[|\]|\.|,|;|:|\?|<|>|=",
        kind: "punctuation",
        keep: true,
        modification: StateModification::None,
    },
    LexerRule {
        pattern: r#"[^\s\w$\{\}\(\)\[\]\.,;:?<>=]+"#,
        kind: "irrelevant",
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
            "class_open" => CLASS_OPEN.to_vec(),
            "class_body" => CLASS_BODY.to_vec(),
            "type_object_open" => TYPE_OBJECT_OPEN.to_vec(),
            "type_object_body" => TYPE_OBJECT_BODY.to_vec(),
        },
        "statements",
    )?;
    parse_at_anchors(
        Module {
            path: ModulePath {
                segments: Vec::new(),
            },
            functions: Vec::new(),
            types: Vec::new(),
        },
        map! {
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "function" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_keyworded_function_definition(lexer)
                            .map(ParsedFeature::Function)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "default" },
                AnchorLocation::Exact { token_kind: "keyword", text: "function" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_default_keyworded_anonymous_function(lexer)
                            .map(ParsedFeature::Function)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "const" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_bound_function_definition(lexer)
                            .map(ParsedFeature::Function)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "let" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_bound_function_definition(lexer)
                            .map(ParsedFeature::Function)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "var" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_bound_function_definition(lexer)
                            .map(ParsedFeature::Function)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "class" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_class_definition(lexer)
                            .map(ParsedFeature::Type)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
                ],
                reducer: Box::new(reduce_parsed_feature),
                _input: std::marker::PhantomData,
            },
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact { token_kind: "keyword", text: "export" },
                AnchorLocation::Exact { token_kind: "keyword", text: "type" },
            ]) => AnchorRule {
                parsers: vec![
                    Box::new(|lexer: &mut LazyStatefulLexer<'_>| {
                        exported_type_alias_definition(lexer)
                            .map(ParsedFeature::Type)
                    }) as Parser<LazyStatefulLexer<'_>, ParsedFeature>,
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
        parse(input).unwrap_or_else(|e| panic!("parse failed: {:?}", e))
    }

    fn find_function<'a>(module: &'a Module, name: &str) -> &'a FunctionDefinition {
        module
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("missing function: {name}"))
    }

    fn find_type<'a>(module: &'a Module, name: &str) -> &'a TypeDefinition {
        module
            .types
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("missing type: {name}"))
    }

    fn assert_is_primitive_number(value: &Type) {
        match value {
            Type::Primitive(PrimitiveType::Number) => {}
            _ => panic!("expected number primitive"),
        }
    }

    fn assert_is_primitive_string(value: &Type) {
        match value {
            Type::Primitive(PrimitiveType::String) => {}
            _ => panic!("expected string primitive"),
        }
    }

    fn assert_is_composite(value: &Type, name: &str) {
        match value {
            Type::Composite(path) if path.name == name => {}
            _ => panic!("expected composite type {name}"),
        }
    }

    #[test]
    fn parses_exported_named_and_variable_bound_functions_only() {
        let input = r#"
            function hidden(a: string): string { return a; }

            export function trim_whitespace(str: string): string {
                return str.trim();
            }

            export const give_back_typesafe = <T>(value: T): T => {
                return value;
            }

            export const via_keyword = function<K>(value: K): K {
                return value;
            }

            const hidden_arrow = (n: number): number => n;
        "#;

        let module = parse_or_panic(input);
        assert_eq!(module.functions.len(), 3);

        let trim = find_function(&module, "trim_whitespace");
        assert_eq!(trim.callable.positional_parameters.len(), 1);
        assert_is_primitive_string(&trim.callable.positional_parameters[0].r#type);
        assert_is_primitive_string(&trim.callable.return_type);

        let generic_arrow = find_function(&module, "give_back_typesafe");
        assert_eq!(generic_arrow.callable.type_parameters.len(), 1);
        assert_eq!(generic_arrow.callable.type_parameters[0].name, "T");
        assert_is_composite(&generic_arrow.callable.return_type, "T");

        let generic_keyword = find_function(&module, "via_keyword");
        assert_eq!(generic_keyword.callable.type_parameters.len(), 1);
        assert_eq!(generic_keyword.callable.type_parameters[0].name, "K");
    }

    #[test]
    fn parses_exported_class_with_properties_methods_and_implements() {
        let input = r#"
            export class Point {
                x: number;
                y: number;

                constructor(x: number, y: number) {
                    this.x = x;
                    this.y = y;
                }

                static from_tuple(tuple: [number, number]): Point {
                    return new Point(tuple[0], tuple[1]);
                }

                distance_to_origin(): number {
                    return Math.sqrt(this.x * this.x + this.y * this.y);
                }
            }

            export class WrapperImpl<K, V> implements Wrapper<K, V> {
                x: K;
                y: V;

                constructor(x: K, y: V) {
                    this.x = x;
                    this.y = y;
                }

                some_method(): string {
                    return "ok";
                }
            }
        "#;

        let module = parse_or_panic(input);
        assert_eq!(module.types.len(), 2);

        let point = find_type(&module, "Point");
        assert_eq!(point.properties.len(), 2);
        assert_is_primitive_number(&point.properties[0].1);
        assert!(point.default_constructor.is_some());
        assert_eq!(point.static_methods.len(), 1);
        assert_eq!(point.methods.len(), 1);
        assert_eq!(point.named_constructors.len(), 1);
        assert_eq!(point.named_constructors[0].name, "from_tuple");

        let wrapper_impl = find_type(&module, "WrapperImpl");
        assert_eq!(wrapper_impl.type_parameters.len(), 2);
        assert_eq!(wrapper_impl.implements.len(), 1);
        assert_eq!(wrapper_impl.implements[0].name, "Wrapper");
        assert_eq!(wrapper_impl.properties.len(), 2);
    }

    #[test]
    fn parses_exported_type_alias_object_with_generics_methods_and_properties() {
        let input = r#"
            export type Wrapper<K, V> = {
                x: K;
                y: V;
                some_method(): string;
            }
        "#;

        let module = parse_or_panic(input);
        assert_eq!(module.types.len(), 1);

        let wrapper = find_type(&module, "Wrapper");
        assert_eq!(wrapper.type_parameters.len(), 2);
        assert_eq!(wrapper.properties.len(), 2);
        assert_eq!(wrapper.methods.len(), 1);
        assert_eq!(wrapper.methods[0].name, "some_method");
        assert_is_primitive_string(&wrapper.methods[0].callable.return_type);
    }

    #[test]
    fn ignores_non_exported_and_does_not_parse_inside_function_body() {
        let input = r#"
            export function host(): string {
                const fake = "export class NotAType { x: number }";
                if (true) {
                    const also_fake = "export type Nope = { y: number }";
                }
                return fake;
            }

            class Hidden {
                value: number;
            }

            export const shown = (v: number): number => {
                return v;
            }
        "#;

        let module = parse_or_panic(input);
        let mut function_names = module
            .functions
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<_>>();
        function_names.sort();
        assert_eq!(function_names, vec!["host".to_string(), "shown".to_string()]);
        assert_eq!(module.types.len(), 0);
        let _ = find_function(&module, "host");
        let _ = find_function(&module, "shown");
    }

    #[test]
    fn parses_export_default_anonymous_keyword_function() {
        let input = r#"
            export default function<T>(value: T): T {
                return value;
            }
        "#;

        let module = parse_or_panic(input);
        assert_eq!(module.functions.len(), 1);
        let default_fn = find_function(&module, "default");
        assert_eq!(default_fn.callable.type_parameters.len(), 1);
        assert_eq!(default_fn.callable.type_parameters[0].name, "T");
    }
}
