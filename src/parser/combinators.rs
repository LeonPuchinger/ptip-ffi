use super::lexer::Lexer;
use super::{Parser, ParserError};

pub fn optional<'a, R: 'a>(parser: Parser<'a, R>) -> Parser<'a, Option<R>> {
    Box::new(move |lexer: &mut dyn Lexer<'a>| match parser(lexer) {
        Ok(result) => Ok(Some(result)),
        Err(ParserError::UnexpectedToken { .. }) => Ok(None),
        Err(e) => Err(e),
    })
}
