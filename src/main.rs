use crate::parser::{ParserError, atoms::token_kind, lexer::Lexer};

mod config;
mod languages;
mod parser;

fn main() {
    println!("Hello, world!");

    let languages = languages::register_languages();

    for lang in languages {
        println!("Language found: {}", lang.name);
    }
}

// example parser usage
pub fn custom_parser<'a>(lexer: &mut dyn Lexer<'a>) -> Result<String, ParserError> {
    let identifier = token_kind("Identifier")(lexer)?;
    let _assignment_operator = token_kind("AssignmentOperator")(lexer)?;
    let number = token_kind("Number")(lexer)?;
    Ok(format!("Parsed assignment: {} = {}", identifier, number))
}
