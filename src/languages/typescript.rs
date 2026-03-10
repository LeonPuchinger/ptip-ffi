use crate::{
    config::LanguageConfig,
    features::LanguageFeature,
    parser::{
        ParserError,
        atoms::{exact, token_kind},
        lexer::{self, LazyLexer},
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

pub fn register() -> LanguageConfig<'static> {
    LanguageConfig {
        name: "TypeScript",
        build_lexer: Box::new(|input| Box::new(LazyLexer::new(input, vec![]))),
        parser: Box::new(function_definitions),
    }
}
