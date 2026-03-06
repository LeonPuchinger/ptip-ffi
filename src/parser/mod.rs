pub mod atoms;
pub mod combinators;
pub mod lexer;

use lexer::{Lexer, LexerError};

#[derive(Debug)]
pub enum ParserError {
    UnexpectedToken { expected: String, found: String },
    UnexpectedEof,
    InternalError(String),
    Custom(String),
}

impl From<LexerError> for ParserError {
    fn from(e: LexerError) -> Self {
        match e {
            LexerError::NoMatch => ParserError::Custom(String::from(
                "Input could not be tokenized by any of the lexer rules.",
            )),
            LexerError::Eof => ParserError::UnexpectedEof,
            LexerError::InvalidSnapshot { message } => ParserError::InternalError(message),
        }
    }
}

pub type Parser<'a, R> = Box<dyn Fn(&mut dyn Lexer<'a>) -> Result<R, ParserError> + 'a>;
