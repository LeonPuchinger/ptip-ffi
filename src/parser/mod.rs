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
            // TODO: rethink error mapping
            LexerError::InvalidSnapshot { message } => ParserError::InternalError(message),
            LexerError::InvalidState { message } => ParserError::InternalError(message),
            LexerError::InvalidRule { message } => ParserError::InternalError(message),
            LexerError::InvalidDefaultState { message, .. } => ParserError::InternalError(message),
            LexerError::Custom { message } => ParserError::Custom(message),
        }
    }
}

pub type Parser<'p, R> = Box<dyn for<'a> Fn(&mut dyn Lexer<'a>) -> Result<R, ParserError> + 'p>;
