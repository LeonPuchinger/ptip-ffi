use crate::parser::{ParserError, atoms::token_kind, lexer::Lexer};

mod parser;

fn main() {
    println!("Hello, world!");
}

// example parser usage
pub fn custom_parser<'a>(lexer: &mut dyn Lexer<'a>) -> Result<String, ParserError> {
    let identifier = token_kind("Identifier")(lexer)?;
    let _assignment_operator = token_kind("AssignmentOperator")(lexer)?;
    let number = token_kind("Number")(lexer)?;
    Ok(format!("Parsed assignment: {} = {}", identifier, number))
}
