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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::atoms;
    use crate::parser::lexer::{LexerError, LexerState, Token, TokenPosition};
    use std::collections::HashMap;

    struct TestLexer<'a> {
        tokens: Vec<Token<'a>>,
        index: usize,
        cursor: usize,
    }

    impl<'a> TestLexer<'a> {
        fn new(tokens: Vec<Token<'a>>) -> Self {
            Self {
                tokens,
                index: 0,
                cursor: 0,
            }
        }
    }

    impl<'a> Lexer<'a> for TestLexer<'a> {
        fn next(&mut self) -> Result<Token<'a>, LexerError> {
            if self.index >= self.tokens.len() {
                return Err(LexerError::Eof);
            }
            let token = self.tokens[self.index].clone();
            self.index += 1;
            self.cursor += 1;
            Ok(token)
        }

        fn snapshot(&self) -> LexerState {
            LexerState {
                token_buffer_index: self.index,
                input_cursor: self.cursor,
                input_row: 0,
                input_column: 0,
            }
        }

        fn restore(&mut self, state: LexerState) -> Option<LexerError> {
            self.index = state.token_buffer_index;
            self.cursor = state.input_cursor;
            None
        }
    }

    fn tok(kind: &'static str, text: &'static str) -> Token<'static> {
        Token {
            kind,
            text,
            position: TokenPosition {
                row_begin: 0,
                row_end: 0,
                column_begin: 0,
                column_end: 0,
            },
        }
    }

    #[test]
    fn optional_wraps_success_in_some() {
        let mut lexer = TestLexer::new(vec![tok("K", "hello")]);
        let parser = optional(atoms::token_kind("K"));
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, Some("hello".to_string()));
        assert_eq!(lexer.snapshot().token_buffer_index, 1);
    }

    #[test]
    fn optional_turns_unexpected_token_into_none_and_does_not_consume() {
        let mut lexer = TestLexer::new(vec![tok("K", "hello")]);
        let parser = optional(atoms::token_kind("Other"));
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, None);
        assert_eq!(lexer.snapshot().token_buffer_index, 0);
    }

    #[test]
    fn optional_propagates_non_unexpected_errors() {
        let mut lexer = TestLexer::new(vec![]);
        let parser: Parser<'static, ()> = Box::new(|_| Err(ParserError::Custom("boom".into())));
        let opt = optional(parser);

        let err = opt(&mut lexer).unwrap_err();
        match err {
            ParserError::Custom(msg) => assert_eq!(msg, "boom"),
            other => panic!("expected Custom error, got: {:?}", other),
        }
    }

    #[test]
    fn parse_at_anchors_collects_features_and_advances_stream() {
        let mut anchors: HashMap<AnchorLocation<'static>, Vec<Parser<'static, String>>> =
            HashMap::new();
        anchors.insert(
            AnchorLocation {
                token_kind: "A",
                text: "@",
            },
            vec![Box::new(|lexer: &mut dyn Lexer<'_>| {
                let t1 = lexer.next()?;
                if t1.kind != "A" || t1.text != "@" {
                    return Err(ParserError::UnexpectedToken {
                        expected: "A:@".into(),
                        found: format!("{}:{}", t1.kind, t1.text),
                    });
                }
                let t2 = lexer.next()?;
                if t2.kind != "V" {
                    return Err(ParserError::UnexpectedToken {
                        expected: "V".into(),
                        found: t2.kind.into(),
                    });
                }
                Ok(t2.text.to_string())
            })],
        );

        let mut lexer = TestLexer::new(vec![
            tok("N", "x"),
            tok("A", "@"),
            tok("V", "one"),
            tok("N", "y"),
            tok("A", "@"),
            tok("V", "two"),
        ]);

        let parser = parse_at_anchors(anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(lexer.snapshot().token_buffer_index, 6);
    }

    #[test]
    fn parse_at_anchors_tries_parsers_in_order_and_restores_between_attempts() {
        let mut anchors: HashMap<AnchorLocation<'static>, Vec<Parser<'static, &'static str>>> =
            HashMap::new();

        let parser_consumes_then_fails: Parser<'static, &'static str> =
            Box::new(|lexer: &mut dyn Lexer<'_>| {
                // Consume the anchor token but fail without restoring.
                let _ = lexer.next()?;
                Err(ParserError::UnexpectedToken {
                    expected: "something else".into(),
                    found: "anchor".into(),
                })
            });

        let parser_succeeds: Parser<'static, &'static str> =
            Box::new(|lexer: &mut dyn Lexer<'_>| {
                let t = lexer.next()?;
                if t.kind == "A" && t.text == "@" {
                    Ok("hit")
                } else {
                    Err(ParserError::UnexpectedToken {
                        expected: "A:@".into(),
                        found: format!("{}:{}", t.kind, t.text),
                    })
                }
            });

        anchors.insert(
            AnchorLocation {
                token_kind: "A",
                text: "@",
            },
            vec![parser_consumes_then_fails, parser_succeeds],
        );

        let mut lexer = TestLexer::new(vec![tok("A", "@"), tok("N", "tail")]);
        let parser = parse_at_anchors(anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["hit"]);
        assert_eq!(lexer.snapshot().token_buffer_index, 2);
    }

    #[test]
    fn parse_at_anchors_advances_by_one_when_anchor_parser_does_not_match() {
        let mut anchors: HashMap<AnchorLocation<'static>, Vec<Parser<'static, String>>> =
            HashMap::new();
        anchors.insert(
            AnchorLocation {
                token_kind: "A",
                text: "@",
            },
            vec![Box::new(|lexer: &mut dyn Lexer<'_>| {
                let t = lexer.next()?;
                Err(ParserError::UnexpectedToken {
                    expected: "never".into(),
                    found: format!("{}:{}", t.kind, t.text),
                })
            })],
        );

        // First anchor won't match; second anchor should still be found.
        let mut lexer = TestLexer::new(vec![
            tok("A", "@"),
            tok("N", "noise"),
            tok("A", "@"),
            tok("V", "ok"),
        ]);

        let mut anchors2 = anchors;
        anchors2.insert(
            AnchorLocation {
                token_kind: "A",
                text: "@",
            },
            vec![Box::new(|lexer: &mut dyn Lexer<'_>| {
                let t1 = lexer.next()?;
                if t1.kind != "A" {
                    return Err(ParserError::UnexpectedToken {
                        expected: "A".into(),
                        found: t1.kind.into(),
                    });
                }
                if let Ok(t2) = lexer.peek()
                    && t2.kind == "V"
                {
                    let t2 = lexer.next()?;
                    return Ok(t2.text.to_string());
                }
                Err(ParserError::UnexpectedToken {
                    expected: "V".into(),
                    found: "not V".into(),
                })
            })],
        );

        let parser = parse_at_anchors(anchors2);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["ok".to_string()]);
    }

    #[test]
    fn parse_at_anchors_errors_if_successful_parser_consumes_no_tokens() {
        let mut anchors: HashMap<AnchorLocation<'static>, Vec<Parser<'static, ()>>> =
            HashMap::new();
        anchors.insert(
            AnchorLocation {
                token_kind: "A",
                text: "@",
            },
            vec![Box::new(|_lexer: &mut dyn Lexer<'_>| Ok(()))],
        );

        let mut lexer = TestLexer::new(vec![tok("A", "@"), tok("N", "tail")]);
        let parser = parse_at_anchors(anchors);
        let err = parser(&mut lexer).unwrap_err();

        match err {
            ParserError::Custom(msg) => assert!(msg.contains("did not consume any tokens")),
            other => panic!("expected Custom error, got: {:?}", other),
        }
    }
}
