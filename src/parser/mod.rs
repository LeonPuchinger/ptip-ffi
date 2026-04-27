pub(crate) mod atoms;
pub(crate) mod combinators;
pub(crate) mod lexer;

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

#[allow(type_alias_bounds)]
pub type Parser<'parser, 'input, L: Lexer<'input>, R> =
    Box<dyn Fn(&mut L) -> Result<R, ParserError> + 'parser>;
