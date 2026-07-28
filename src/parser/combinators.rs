use std::collections::HashMap;
use std::marker::PhantomData;

use crate::parser::Parser;
use crate::parser::lexer::LexerError;

use super::ParserError;
use super::lexer::{Lexer, LexerState, Token};

/// Turns any parser into a parser that cannot fail. When the nested parser
/// succeeds, its result is wrapped in `Some`. If the nested parser encounters
/// an unexpected token, returns `None` instead. Also, in the latter case,
/// the input stream remains unchanged.
pub fn optional<'p, 'input, L, R>(
    parser: Parser<'p, 'input, L, R>,
) -> Parser<'p, 'input, L, Option<R>>
where
    L: Lexer<'input> + 'p,
    R: 'p,
{
    Box::new(move |lexer: &mut L| {
        let snapshot = lexer.snapshot();
        match parser(lexer) {
            Ok(result) => Ok(Some(result)),
            Err(ParserError::UnexpectedToken { .. }) => {
                lexer.restore(&snapshot);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    })
}

/// Represents a specific location in the token stream that can be used as an
/// anchor for parsing.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AnchorLocation<'a> {
    /// Anchor based on token kind only (e.g. any "identifier" token).
    Kind(&'a str),
    /// Anchor based on both token kind and exact text (e.g. a "keyword" token
    /// with text "function").
    Exact { token_kind: &'a str, text: &'a str },
    /// Anchor based on a series of token locations.
    Consecutive(Vec<AnchorLocation<'a>>),
}

impl AnchorLocation<'_> {
    pub fn matches_token(&self, token: &Token) -> bool {
        match self {
            AnchorLocation::Kind(kind) => token.kind == *kind,
            AnchorLocation::Exact { token_kind, text } => {
                token.kind == *token_kind && token.text == *text
            }
            AnchorLocation::Consecutive(_) => false,
        }
    }
}

/// Parsing behavior for one anchor location.
pub struct AnchorRule<'p, 'input, L, A, R>
where
    L: Lexer<'input> + ?Sized + 'p,
    A: 'p,
    R: 'p,
{
    pub parsers: Vec<Parser<'p, 'input, L, R>>,
    pub reducer: Box<dyn Fn(A, R) -> A + 'p>,
    pub _input: PhantomData<&'input ()>,
}

/// Used to keep track of a consecutive anchor candidate that
/// is currently being matched in the token stream.
#[derive(Clone)]
struct PendingConsecutive<'anchor, 'input> {
    location: AnchorLocation<'anchor>,
    start_snapshot: LexerState<'input>,
    start_token: Token<'input>,
    progress: usize,
}

fn try_rule<'p, 'input, L, A, R>(
    lexer: &mut L,
    rule: &AnchorRule<'p, 'input, L, A, R>,
    before_anchor: &LexerState<'input>,
    matched_token: &Token<'input>,
    accumulator: A,
) -> Result<Option<A>, ParserError>
where
    L: Lexer<'input> + ?Sized + 'p,
    A: 'p,
    R: 'p,
{
    let mut accumulator = accumulator;
    for parser in &rule.parsers {
        lexer.restore(before_anchor);
        if let Ok(feature) = parser(lexer) {
            accumulator = (rule.reducer)(accumulator, feature);
            if lexer.snapshot().input_cursor == before_anchor.input_cursor {
                return Err(ParserError::Custom(format!(
                    "Parser for anchor {:?} did not consume any tokens",
                    (matched_token.kind, matched_token.text)
                )));
            }
            return Ok(Some(accumulator));
        }
    }
    lexer.restore(before_anchor);
    Ok(None)
}

/// Returns a parser that iterates over the token stream, looking for tokens that
/// match any of the provided anchor locations. When such a token is found, the
/// associated parsers are attempted. If a parser successfully matches, its result
/// is folded into the accumulator with the anchor's reducer. If no parser matches
/// or the token is not an anchor, the lexer advances by one token and continues
/// searching.
pub fn parse_at_anchors<'p, 'input, 'anchor, L, A, R>(
    initial: A,
    anchors: HashMap<AnchorLocation<'anchor>, AnchorRule<'p, 'input, L, A, R>>,
) -> Parser<'p, 'input, L, A>
where
    'anchor: 'p,
    'input: 'p,
    L: Lexer<'input> + ?Sized + 'p,
    A: Clone + 'p,
    R: 'p,
{
    Box::new(move |lexer: &mut L| {
        let mut accumulator = initial.clone();
        let mut pending_consecutive: Option<PendingConsecutive<'anchor, 'input>> = None;
        'anchor: loop {
            let next = match lexer.peek() {
                Ok(token) => token,
                Err(LexerError::Eof) => break 'anchor,
                Err(error) => return Err(error.into()),
            };
            if let Some(rule) = anchors
                .get(&AnchorLocation::Exact {
                    token_kind: next.kind,
                    text: next.text,
                })
                .or_else(|| anchors.get(&AnchorLocation::Kind(next.kind)))
            {
                pending_consecutive = None;
                let before_anchor = lexer.snapshot();
                if let Some(updated_accumulator) =
                    try_rule(lexer, rule, &before_anchor, &next, accumulator.clone())?
                {
                    accumulator = updated_accumulator;
                    continue 'anchor;
                }
            }

            let before_current = lexer.snapshot();
            let mut start_location: Option<AnchorLocation<'anchor>> = None;
            let mut start_is_exact = false;
            for location in anchors.keys() {
                if let AnchorLocation::Consecutive(sequence) = location
                    && let Some(first) = sequence.first()
                    && first.matches_token(&next)
                {
                    let is_exact = matches!(first, AnchorLocation::Exact { .. });
                    let replace_start = start_location.is_none() || (is_exact && !start_is_exact);
                    if replace_start {
                        start_location = Some(location.clone());
                        start_is_exact = is_exact;
                        if is_exact {
                            break;
                        }
                    }
                }
            }

            if let Some(location) = start_location {
                pending_consecutive = None;
                let candidate = PendingConsecutive {
                    location,
                    start_snapshot: before_current.clone(),
                    start_token: next.clone(),
                    progress: 1,
                };

                if let AnchorLocation::Consecutive(sequence) = &candidate.location
                    && sequence.len() == 1
                {
                    let Some(rule) = anchors.get(&candidate.location) else {
                        return Err(ParserError::Custom(
                            "Consecutive anchor disappeared while being processed".into(),
                        ));
                    };
                    if let Some(updated_accumulator) = try_rule(
                        lexer,
                        rule,
                        &candidate.start_snapshot,
                        &candidate.start_token,
                        accumulator.clone(),
                    )? {
                        accumulator = updated_accumulator;
                        continue 'anchor;
                    }
                    lexer.next()?;
                    continue 'anchor;
                }

                lexer.next()?;
                pending_consecutive = Some(candidate);
                continue 'anchor;
            }

            if let Some(candidate) = pending_consecutive.as_mut() {
                let mut advanced = false;
                let mut completed = false;
                if let AnchorLocation::Consecutive(sequence) = &candidate.location
                    && candidate.progress < sequence.len()
                {
                    let expected = &sequence[candidate.progress];
                    if expected.matches_token(&next) {
                        lexer.next()?;
                        candidate.progress += 1;
                        advanced = true;
                        completed = candidate.progress == sequence.len();
                    }
                }

                if completed {
                    let candidate = pending_consecutive.take().expect("candidate must exist");
                    let Some(rule) = anchors.get(&candidate.location) else {
                        return Err(ParserError::Custom(
                            "Consecutive anchor disappeared while being processed".into(),
                        ));
                    };
                    if let Some(updated_accumulator) = try_rule(
                        lexer,
                        rule,
                        &candidate.start_snapshot,
                        &candidate.start_token,
                        accumulator.clone(),
                    )? {
                        accumulator = updated_accumulator;
                        continue 'anchor;
                    }

                    lexer.restore(&candidate.start_snapshot);
                    pending_consecutive = None;
                    match lexer.next() {
                        Ok(_) => continue,
                        Err(LexerError::Eof) => break 'anchor,
                        Err(e) => return Err(e.into()),
                    }
                }

                if advanced {
                    continue 'anchor;
                }

                pending_consecutive = None;
            }

            match lexer.next() {
                Ok(_) => continue,
                Err(LexerError::Eof) => break 'anchor,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(accumulator)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::atoms;
    use crate::parser::lexer::{LexerError, LexerState, Token, TokenPosition};
    use std::collections::HashMap;
    use std::marker::PhantomData;

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

        fn snapshot(&self) -> LexerState<'a> {
            LexerState::new(self.index, self.cursor, 0, 0, Vec::new())
        }

        fn restore(&mut self, state: &LexerState<'a>) -> Option<LexerError> {
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

    fn anchor_rule<A, R>(
        parsers: Vec<Parser<'static, 'static, TestLexer<'static>, R>>,
        reducer: impl Fn(A, R) -> A + 'static,
    ) -> AnchorRule<'static, 'static, TestLexer<'static>, A, R> {
        AnchorRule {
            parsers,
            reducer: Box::new(reducer),
            _input: PhantomData,
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
        let parser: Parser<'static, 'static, TestLexer<'static>, ()> =
            Box::new(|_: &mut TestLexer<'static>| Err(ParserError::Custom("boom".into())));
        let opt = optional(parser);

        let err = opt(&mut lexer).unwrap_err();
        match err {
            ParserError::Custom(msg) => assert_eq!(msg, "boom"),
            other => panic!("expected Custom error, got: {:?}", other),
        }
    }

    #[test]
    fn parse_at_anchors_collects_features_and_advances_stream() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<String>, String>,
        > = HashMap::new();
        anchors.insert(
            AnchorLocation::Exact {
                token_kind: "A",
                text: "@",
            },
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
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
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let mut lexer = TestLexer::new(vec![
            tok("N", "x"),
            tok("A", "@"),
            tok("V", "one"),
            tok("N", "y"),
            tok("A", "@"),
            tok("V", "two"),
        ]);

        let parser = parse_at_anchors(Vec::new(), anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(lexer.snapshot().token_buffer_index, 6);
    }

    #[test]
    fn parse_at_anchors_supports_kind_only_anchors() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<String>, String>,
        > = HashMap::new();
        anchors.insert(
            AnchorLocation::Kind("A"),
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let anchor = lexer.next()?;
                    if anchor.kind != "A" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "A".into(),
                            found: anchor.kind.into(),
                        });
                    }
                    let value = lexer.next()?;
                    if value.kind != "V" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "V".into(),
                            found: value.kind.into(),
                        });
                    }
                    Ok(value.text.to_string())
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let mut lexer = TestLexer::new(vec![
            tok("N", "x"),
            tok("A", "@"),
            tok("V", "one"),
            tok("N", "y"),
            tok("A", "#"),
            tok("V", "two"),
        ]);

        let parser = parse_at_anchors(Vec::new(), anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(lexer.snapshot().token_buffer_index, 6);
    }

    #[test]
    fn parse_at_anchors_prefers_exact_anchor_over_kind_anchor() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<String>, String>,
        > = HashMap::new();

        anchors.insert(
            AnchorLocation::Kind("A"),
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let anchor = lexer.next()?;
                    if anchor.kind != "A" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "A".into(),
                            found: anchor.kind.into(),
                        });
                    }
                    let value = lexer.next()?;
                    if value.kind != "V" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "V".into(),
                            found: value.kind.into(),
                        });
                    }
                    Ok(format!("kind:{}", value.text))
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        anchors.insert(
            AnchorLocation::Exact {
                token_kind: "A",
                text: "@",
            },
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let anchor = lexer.next()?;
                    if anchor.kind != "A" || anchor.text != "@" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "A:@".into(),
                            found: format!("{}:{}", anchor.kind, anchor.text),
                        });
                    }
                    let value = lexer.next()?;
                    if value.kind != "V" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "V".into(),
                            found: value.kind.into(),
                        });
                    }
                    Ok(format!("exact:{}", value.text))
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let mut lexer = TestLexer::new(vec![
            tok("A", "@"),
            tok("V", "one"),
            tok("A", "#"),
            tok("V", "two"),
        ]);

        let parser = parse_at_anchors(Vec::new(), anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(
            result,
            vec!["exact:one".to_string(), "kind:two".to_string()]
        );
        assert_eq!(lexer.snapshot().token_buffer_index, 4);
    }

    #[test]
    fn parse_at_anchors_tries_parsers_in_order_and_restores_between_attempts() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<&'static str>, &'static str>,
        > = HashMap::new();

        let parser_consumes_then_fails: Parser<'static, 'static, TestLexer<'static>, &'static str> =
            Box::new(|lexer: &mut TestLexer<'static>| {
                // Consume the anchor token but fail without restoring.
                let _ = lexer.next()?;
                Err(ParserError::UnexpectedToken {
                    expected: "something else".into(),
                    found: "anchor".into(),
                })
            });

        let parser_succeeds: Parser<'static, 'static, TestLexer<'static>, &'static str> =
            Box::new(|lexer: &mut TestLexer<'static>| {
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
            AnchorLocation::Exact {
                token_kind: "A",
                text: "@",
            },
            anchor_rule(
                vec![parser_consumes_then_fails, parser_succeeds],
                |mut features: Vec<&'static str>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let mut lexer = TestLexer::new(vec![tok("A", "@"), tok("N", "tail")]);
        let parser = parse_at_anchors(Vec::new(), anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["hit"]);
        assert_eq!(lexer.snapshot().token_buffer_index, 2);
    }

    #[test]
    fn parse_at_anchors_advances_by_one_when_anchor_parser_does_not_match() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<String>, String>,
        > = HashMap::new();
        anchors.insert(
            AnchorLocation::Exact {
                token_kind: "A",
                text: "@",
            },
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let t = lexer.next()?;
                    Err(ParserError::UnexpectedToken {
                        expected: "never".into(),
                        found: format!("{}:{}", t.kind, t.text),
                    })
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
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
            AnchorLocation::Exact {
                token_kind: "A",
                text: "@",
            },
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
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
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let parser = parse_at_anchors(Vec::new(), anchors2);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["ok".to_string()]);
    }

    #[test]
    fn parse_at_anchors_errors_if_successful_parser_consumes_no_tokens() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, (), ()>,
        > = HashMap::new();
        anchors.insert(
            AnchorLocation::Exact {
                token_kind: "A",
                text: "@",
            },
            anchor_rule(
                vec![Box::new(|_lexer: &mut TestLexer<'static>| Ok(()))],
                |acc, _| acc,
            ),
        );

        let mut lexer = TestLexer::new(vec![tok("A", "@"), tok("N", "tail")]);
        let parser = parse_at_anchors((), anchors);
        let err = parser(&mut lexer).unwrap_err();

        match err {
            ParserError::Custom(msg) => assert!(msg.contains("did not consume any tokens")),
            other => panic!("expected Custom error, got: {:?}", other),
        }
    }

    #[test]
    fn parse_at_anchors_supports_consecutive_anchors() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<String>, String>,
        > = HashMap::new();
        anchors.insert(
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact {
                    token_kind: "A",
                    text: "@",
                },
                AnchorLocation::Kind("V"),
            ]),
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let start = lexer.next()?;
                    if start.kind != "A" || start.text != "@" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "A:@".into(),
                            found: format!("{}:{}", start.kind, start.text),
                        });
                    }
                    let value = lexer.next()?;
                    if value.kind != "V" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "V".into(),
                            found: value.kind.into(),
                        });
                    }
                    Ok(value.text.to_string())
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let mut lexer = TestLexer::new(vec![
            tok("N", "x"),
            tok("A", "@"),
            tok("V", "one"),
            tok("N", "y"),
            tok("A", "@"),
            tok("V", "two"),
        ]);

        let parser = parse_at_anchors(Vec::new(), anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(result, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(lexer.snapshot().token_buffer_index, 6);
    }

    #[test]
    fn parse_at_anchors_drops_consecutive_anchor_when_competing_anchor_matches() {
        let mut anchors: HashMap<
            AnchorLocation<'static>,
            AnchorRule<'static, 'static, TestLexer<'static>, Vec<String>, String>,
        > = HashMap::new();

        anchors.insert(
            AnchorLocation::Consecutive(vec![
                AnchorLocation::Exact {
                    token_kind: "A",
                    text: "@",
                },
                AnchorLocation::Kind("B"),
            ]),
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let start = lexer.next()?;
                    if start.kind != "A" || start.text != "@" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "A:@".into(),
                            found: format!("{}:{}", start.kind, start.text),
                        });
                    }
                    let value = lexer.next()?;
                    if value.kind != "B" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "B".into(),
                            found: value.kind.into(),
                        });
                    }
                    Ok(format!("consecutive:{}", value.text))
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        anchors.insert(
            AnchorLocation::Kind("V"),
            anchor_rule(
                vec![Box::new(|lexer: &mut TestLexer<'static>| {
                    let value = lexer.next()?;
                    if value.kind != "V" {
                        return Err(ParserError::UnexpectedToken {
                            expected: "V".into(),
                            found: value.kind.into(),
                        });
                    }
                    Ok(format!("single:{}", value.text))
                })],
                |mut features: Vec<String>, feature| {
                    features.push(feature);
                    features
                },
            ),
        );

        let mut lexer = TestLexer::new(vec![
            tok("A", "@"),
            tok("V", "one"),
            tok("A", "@"),
            tok("B", "ok"),
        ]);

        let parser = parse_at_anchors(Vec::new(), anchors);
        let result = parser(&mut lexer).unwrap();

        assert_eq!(
            result,
            vec!["single:one".to_string(), "consecutive:ok".to_string()]
        );
        assert_eq!(lexer.snapshot().token_buffer_index, 4);
    }
}
