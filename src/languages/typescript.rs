use crate::{
    config::{FeatureParser, LanguageConfig, LanguageFeature},
    parser::{
        ParserError,
        atoms::{exact, token_kind},
        lexer::{self, LazyLexer},
    },
};

fn keyworded_function_definition(lexer: &mut dyn lexer::Lexer) -> Result<(), ParserError> {
    let _keyword = exact("function")(lexer)?;
    let _name = token_kind("identifier")(lexer)?;
    let _open_parenthesis = exact("(")(lexer)?;
    let _close_parenthesis = exact(")")(lexer)?;
    Ok(())
}

pub fn register<'a>() -> LanguageConfig<'a> {
    // Dummy config for TS, just for demonstration purposes of the overall architecture.
    LanguageConfig {
        name: "TypeScript",
        features: vec![
            FeatureParser {
                kind: LanguageFeature::Function,
                lexer: LazyLexer::new(
                    "", // TODO: Pass a lexer factory here that passes the input to the lexer later
                    vec![
                        // Define lexer rules for TypeScript function syntax
                    ],
                ),
                parser: Box::new(|lexer| {
                    // Implement a parser for TypeScript function declarations
                    Ok(String::from("Parsed TypeScript function"))
                }),
            },
            FeatureParser {
                kind: LanguageFeature::Type,
                lexer: LazyLexer::new(
                    "",
                    vec![
                        // Define lexer rules for TypeScript type syntax
                    ],
                ),
                parser: Box::new(|lexer| {
                    // Implement a parser for TypeScript type declarations
                    Ok(String::from("Parsed TypeScript type"))
                }),
            },
        ],
    }
}
