use crate::parser::{ParserError, lexer::LexerError};

#[derive(Debug)]
pub enum PTIPFFIError {
    LexerError(LexerError),
    ParserError(ParserError),
    LanguageNotFound(String),
    OutputPersistenceError(std::io::Error),
}

impl From<LexerError> for PTIPFFIError {
    fn from(e: LexerError) -> Self {
        PTIPFFIError::LexerError(e)
    }
}

impl From<ParserError> for PTIPFFIError {
    fn from(e: ParserError) -> Self {
        PTIPFFIError::ParserError(e)
    }
}

impl From<std::io::Error> for PTIPFFIError {
    fn from(e: std::io::Error) -> Self {
        PTIPFFIError::OutputPersistenceError(e)
    }
}
