use crate::parser::Parser;
use crate::parser::lexer::LexerError;

use super::ParserError;
use super::lexer::Lexer;

/// Turns any parser into a parser that cannot fail. When the nested parser
/// succeeds, its result is wrapped in `Some`. If the nested parser encounters
/// an unexpected token, returns `None` instead.
pub fn optional<'a, R: 'a>(parser: Parser<'a, R>) -> Parser<'a, Option<R>> {
    Box::new(move |lexer: &mut dyn Lexer<'_>| match parser(lexer) {
        Ok(result) => Ok(Some(result)),
        Err(ParserError::UnexpectedToken { .. }) => Ok(None),
        Err(e) => Err(e),
    })
}

/// Represents a specific location in the token stream that can be used as an anchor for parsing.
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct AnchorLocation<'a> {
    pub token_kind: &'a str,
    pub text: &'a str,
}

/// Returns a parser that iterates over the token stream, looking for tokens that
/// match any of the provided anchor locations. When such a token is found, the
/// associated parsers are attempted. If a parser successfully matches, its result
/// is added to the list of return values. If no parser matches or the token is
/// not an anchor, the lexer advances by one token and continues searching.
pub fn parse_at_anchors<'a, R: 'a>(
    anchors: std::collections::HashMap<AnchorLocation<'static>, Vec<Parser<'a, R>>>,
) -> Parser<'a, Vec<R>> {
    Box::new(move |lexer: &mut dyn Lexer<'_>| {
        let mut features = Vec::new();
        'anchor: loop {
            let next = match lexer.peek() {
                Ok(token) => token,
                Err(LexerError::Eof) => break 'anchor,
                Err(error) => return Err(error.into()),
            };
            if let Some(parsers) = anchors.get(&AnchorLocation {
                token_kind: next.kind,
                text: next.text,
            }) {
                let before_anchor = lexer.snapshot();
                for parser in parsers {
                    lexer.restore(before_anchor);
                    if let Ok(feature) = parser(lexer) {
                        features.push(feature);
                        // Assert whether the successful parser actually consumed any tokens
                        if lexer.snapshot().input_cursor == before_anchor.input_cursor {
                            return Err(ParserError::Custom(format!(
                                "Parser for anchor {:?} did not consume any tokens",
                                (next.kind, next.text)
                            )));
                        }
                        continue 'anchor;
                    }
                }
                lexer.restore(before_anchor);
            }
            // No parser matched or the token is not an anchor.
            // In either case, the lexer needs to be advanced one token.
            match lexer.next() {
                Ok(_) => continue,
                Err(LexerError::Eof) => break 'anchor,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(features)
    })
}
