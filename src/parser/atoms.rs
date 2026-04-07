use crate::parser::{
    Parser, ParserError,
    lexer::{Lexer, LexerError},
};

/// A parser that matches a token based on its kind.
pub fn token_kind<'a>(expected_kind: &'a str) -> Parser<'a, String> {
    Box::new(move |lexer: &mut dyn Lexer<'_>| {
        let snapshot = lexer.snapshot();
        let token = lexer.next()?;
        if token.kind == expected_kind {
            Ok(token.text.to_string())
        } else {
            lexer.restore(snapshot);
            Err(ParserError::UnexpectedToken {
                expected: expected_kind.to_string(),
                found: token.kind.to_string(),
            })
        }
    })
}

/// A parser that matches a specific sequence of text from the input.
/// It should be mentioned, however, that the expected text has to align with token boundaries.
pub fn exact<'a>(expected_text: &'a str) -> Parser<'a, String> {
    Box::new(move |lexer: &mut dyn Lexer<'_>| {
        let snapshot = lexer.snapshot();
        let mut matched = String::new();
        while matched.len() < expected_text.len() {
            let token = match lexer.next() {
                Ok(t) => t,
                Err(LexerError::Eof) => {
                    lexer.restore(snapshot);
                    return Err(ParserError::UnexpectedEof);
                }
                Err(e) => return Err(e.into()),
            };

            let candidate = format!("{}{}", matched, token.text);
            if expected_text.starts_with(&candidate) {
                matched = candidate;
                if matched == expected_text {
                    return Ok(matched);
                }
            } else {
                matched = candidate;
                break;
            }
        }
        lexer.restore(snapshot);
        Err(ParserError::UnexpectedToken {
            expected: expected_text.to_string(),
            found: matched,
        })
    })
}
