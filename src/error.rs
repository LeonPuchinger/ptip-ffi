use crate::parser::{ParserError, lexer::LexerError};

pub enum PTIPFFIError {
    LexerError(LexerError),
    ParserError(ParserError),
    LanguageNotFound(String),
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
