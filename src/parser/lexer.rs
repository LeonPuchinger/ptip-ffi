use regex::Regex;
use std::{collections::HashMap, vec};

#[derive(Clone, Debug)]
pub struct TokenPosition {
    pub row_begin: usize,
    pub row_end: usize,
    pub column_begin: usize,
    pub column_end: usize,
}

#[derive(Clone, Debug)]
pub struct Token<'a> {
    pub kind: &'a str,
    pub text: &'a str,
    pub position: TokenPosition,
}

/// A state modification controls what happens to the lexer's internal stack of
/// rulesets when a rule is matched. This grants the lexer context-free
/// capabilities, which is necessary for tokenizing complex languages.
#[derive(Clone, Copy)]
pub enum StateModification {
    /// The lexer's stack remains unchanged.
    None,
    /// The top ruleset is popped from the lexer's stack. If the stack only
    /// contains one ruleset, this results in a `LexerError::InvalidState`.
    Pop,
    /// Pushes a new ruleset onto the lexer's stack based on its name.
    Push(&'static str),
}

/// A lexer rule defines which tokens are generated from the input. They also
/// control the state of the lexer by allowing rulesets (states) to be pushed
/// and popped from the lexer's internal stack.
#[derive(Clone)]
pub struct LexerRule {
    pub pattern: &'static str,
    pub kind: &'static str,
    pub keep: bool,
    pub modification: StateModification,
}

/// The lexer compiles regular `LexerRule`s into `CompiledLexerRule`s, which
/// contain compiled regex patterns, for instance. The regular `LexerRule`s can
/// be defined statically, while the compiled variants cannot. The compiled
/// variants are stored in the lexer during runtime and are used for tokenization.
#[derive(Clone)]
struct CompiledLexerRule {
    pattern: Regex,
    kind: &'static str,
    keep: bool,
    modification: StateModification,
}

#[derive(Debug)]
pub enum LexerError {
    NoMatch,
    Eof,
    InvalidSnapshot {
        message: String,
    },
    InvalidState {
        message: String,
    },
    InvalidRule {
        message: String,
    },
    InvalidDefaultState {
        supplied_state: &'static str,
        message: String,
    },
    Custom {
        message: String,
    },
}

impl From<regex::Error> for LexerError {
    fn from(e: regex::Error) -> Self {
        match e {
            regex::Error::Syntax(message) => LexerError::InvalidRule {
                message: format!(
                    "A syntax error was encountered in a rule pattern: '{}'.",
                    message
                ),
            },
            regex::Error::CompiledTooBig(size) => LexerError::InvalidRule {
                message: format!(
                    "A rule pattern is too large to be compiled by the regex engine (size: {}).",
                    size
                ),
            },
            _ => LexerError::Custom {
                message: format!(
                    "An unknown error occurred while compiling the regex: '{:?}'",
                    e
                ),
            },
        }
    }
}

/// A snapshot of the lexer's state, which can be used to restore the lexer to a
/// previous position. The snapshot can be created using `Lexer::snapshot` and
/// restored using `Lexer::restore`.
#[derive(Clone)]
pub struct LexerState {
    pub token_buffer_index: usize,
    pub input_cursor: usize,
    pub input_row: usize,
    pub input_column: usize,
    pub state: Vec<&'static str>,
}

// A pull-based lexer that allows its caller to save and restore its state
// relative to the input sequence.
pub trait Lexer<'a> {
    fn next(&mut self) -> Result<Token<'a>, LexerError>;
    fn snapshot(&self) -> LexerState;
    fn restore(&mut self, state: &LexerState) -> Option<LexerError>;
    fn peek(&mut self) -> Result<Token<'a>, LexerError> {
        let snapshot = self.snapshot();
        let next = self.next();
        if let Some(e) = self.restore(&snapshot) {
            return Err(e);
        }
        next
    }
}

/// A collection of un-compiled lexer rules. To match a token, the lexer only
/// ever looks at a single ruleset, which is the topmost one on the internal stack.
type LexerRuleset = Vec<LexerRule>;

/// Similar to `LexerRuleset`, but it stores `CompiledLexerRule`s
/// which contain compiled regex patterns.
type CompiledLexerRuleset = Vec<CompiledLexerRule>;

/// When a token is emitted from the token buffer of the lexer, the lexer state
/// needs to be updated to reflect that the token has been consumed. For instance,
/// the input cursor needs to be moved forward by the length of the emitted
/// token. This struct represents the necessary information to update the lexer
/// state when a token is emitted from the token buffer alongside the token itself.
struct TokenBufferEntry<'input> {
    token: Token<'input>,
    input_cursor_after: usize,
    input_row_after: usize,
    input_column_after: usize,
    state_modification_after: StateModification,
}

/// A lexer that takes an input string and a set of rules and produces a stream
/// of tokens. The lexer is pull-based and lazy, meaning that it only produces
/// tokens when requested and only processes the minimum amount of input
/// necessary to produce the next token. The lexer maintains an internal buffer
/// of tokens that have been matched so far, so that if the lexer is reset to a
/// previous position, the tokens can be returned from the buffer without having
/// to re-match the input.
/// The lexer also maintains an internal stack of rulesets, which grants it
/// context-free capabilities. When a rule is matched, it can specify a
/// modification to the stack, which allows for rulesets to be pushed and popped
/// from said stack. This is necessary for tokenizing complex languages, which often
/// require different tokenization rules in different contexts
/// (e.g. inside a string literal vs. outside of one).
pub struct LazyStatefulLexer<'input> {
    input: &'input str,
    rulesets: HashMap<&'static str, CompiledLexerRuleset>,
    state: Vec<&'static str>,
    token_buffer: Vec<TokenBufferEntry<'input>>,
    token_buffer_index: usize,
    input_cursor: usize,
    input_row: usize,
    input_column: usize,
}

impl<'input> LazyStatefulLexer<'input> {
    pub fn new(
        input: &'input str,
        rulesets: HashMap<&'static str, LexerRuleset>,
        default: &'static str,
    ) -> Result<Self, LexerError> {
        if !rulesets.contains_key(default) {
            return Err(LexerError::InvalidDefaultState {
                supplied_state: default,
                message: format!(
                    "The default state '{}' could not be found in the provided rulesets.",
                    default
                ),
            });
        }
        let compiled_rulesets = rulesets
            .iter()
            .map(|(&name, rules)| {
                let compiled_rules = rules
                    .iter()
                    .map(|rule| {
                        let pattern = Regex::new(rule.pattern)?;
                        if let StateModification::Push(target) = rule.modification
                            && !rulesets.contains_key(target) {
                                return Err(LexerError::InvalidRule {
                                    message: format!(
                                        "The rule with the pattern '{}' tries to push the ruleset '{}' which is not defined in the provided rulesets.",
                                        rule.pattern, target
                                    ),
                                });
                            }
                        if !rule.keep && !matches!(rule.modification, StateModification::None) {
                            return Err(LexerError::InvalidRule {
                                message: format!(
                                    "The rule with the pattern '{}' tries to modify the lexer state while being marked as 'keep: false', which is not allowed. Only rules that emit a token are allowed to modify the lexer state.",
                                    rule.pattern
                                ),
                            });
                        }
                        Ok(CompiledLexerRule {
                            pattern,
                            kind: rule.kind,
                            keep: rule.keep,
                            modification: rule.modification,
                        })
                    })
                    .collect::<Result<CompiledLexerRuleset, LexerError>>()?;
                Ok((name, compiled_rules))
            })
            .collect::<Result<HashMap<&'static str, CompiledLexerRuleset>, LexerError>>()?;
        let state = vec![default];
        Ok(Self {
            input,
            rulesets: compiled_rulesets,
            state,
            token_buffer: Vec::new(),
            token_buffer_index: 0,
            input_cursor: 0,
            input_row: 0,
            input_column: 0,
        })
    }
}

impl<'input> Lexer<'input> for LazyStatefulLexer<'input> {
    /// Returns the next token from the input. If there are no more tokens,
    /// `LexerError::Eof` is returned. If the next token cannot be matched by
    /// any of the rules, `LexerError::NoMatch` is returned. For each token, the
    /// longest match is chosen. If there are multiple matches of the same
    /// length, the one defined first in the rules is chosen.
    fn next(&mut self) -> Result<Token<'input>, LexerError> {
        if self.token_buffer_index < self.token_buffer.len() {
            // Return the next token from the buffer if available
            let entry = &self.token_buffer[self.token_buffer_index];
            self.token_buffer_index += 1;

            // When replaying buffered tokens, we must also replay the lexer state
            // transitions (cursor movement, line/column tracking, and state stack
            // modification) so that subsequent lexing resumes from the correct
            // position.
            self.input_cursor = entry.input_cursor_after;
            self.input_row = entry.input_row_after;
            self.input_column = entry.input_column_after;
            match entry.state_modification_after {
                StateModification::None => {}
                StateModification::Pop => {
                    if self.state.len() <= 1 {
                        return Err(LexerError::InvalidState {
                            message: String::from(
                                "The lexer state cannot be popped because it only contains one ruleset.",
                            ),
                        });
                    }
                    self.state.pop();
                }
                StateModification::Push(target_ruleset) => {
                    self.state.push(target_ruleset);
                }
            }

            Ok(entry.token.clone())
        } else {
            // Check whether the end of the input has been reached
            if self.input_cursor >= self.input.len() {
                return Err(LexerError::Eof);
            }
            // Try to match next token using the rules. Longest match wins.
            // If there are multiple matches of the same length, the one
            // defined first wins. If there are no matches, an error is returned.
            let remaining = &self.input[self.input_cursor..];
            let mut best_length: usize = 0;
            let mut best_kind: Option<&str> = None;
            let mut best_keep: bool = true;
            let mut best_modification: &StateModification = &StateModification::None;
            let current_ruleset_name = match self.state.last() {
                Some(name) => *name,
                None => {
                    return Err(LexerError::InvalidState {
                        message: String::from(
                            "The lexer is currently not equipped with any rulesets.",
                        ),
                    });
                }
            };
            let current_ruleset = match self.rulesets.get(current_ruleset_name) {
                Some(ruleset) => ruleset,
                None => {
                    return Err(LexerError::InvalidState {
                        message: format!(
                            "The lexer's internal state refers to an unknown ruleset: {}",
                            current_ruleset_name
                        ),
                    });
                }
            };
            for CompiledLexerRule {
                pattern,
                kind,
                keep,
                modification,
            } in current_ruleset.iter()
            {
                // TODO: improve performance by:
                // - anchoring the regexes to the beginning of the string (if not already anchored)
                if let Some(r#match) = pattern.find(remaining)
                    && r#match.start() == 0
                {
                    let matched_length = r#match.end();
                    if matched_length > best_length {
                        best_length = matched_length;
                        best_kind = Some(kind);
                        best_keep = *keep;
                        best_modification = modification;
                    }
                }
            }
            if best_kind.is_none() {
                return Err(LexerError::NoMatch);
            }
            let matched_text = &remaining[..best_length];
            // Calculate the token position
            let row_begin = self.input_row;
            let column_begin = self.input_column;
            let mut row_end = self.input_row;
            let mut column_end = self.input_column;
            for character in matched_text.chars() {
                if character == '\n' {
                    row_end += 1;
                    column_end = 0;
                } else {
                    column_end += 1;
                }
            }
            // Modify the lexer state
            self.input_cursor += best_length;
            self.input_row = row_end;
            self.input_column = column_end;
            match best_modification {
                StateModification::None => {}
                StateModification::Pop => {
                    if self.state.len() <= 1 {
                        return Err(LexerError::InvalidState {
                            message: String::from(
                                "The lexer state cannot be popped because it only contains one ruleset.",
                            ),
                        });
                    }
                    self.state.pop();
                }
                StateModification::Push(target_ruleset) => {
                    self.state.push(target_ruleset);
                }
            }
            if !best_keep {
                // If the token should be skipped, return the next token instead
                return self.next();
            }
            // Construct the new token
            let position = TokenPosition {
                row_begin,
                row_end,
                column_begin,
                column_end,
            };
            let new_token = Token {
                kind: best_kind.unwrap(),
                text: matched_text,
                position,
            };
            // Modify the token buffer
            self.token_buffer.push(TokenBufferEntry {
                token: new_token.clone(),
                input_cursor_after: self.input_cursor,
                input_row_after: self.input_row,
                input_column_after: self.input_column,
                state_modification_after: *best_modification,
            });
            self.token_buffer_index += 1;
            Ok(new_token)
        }
    }

    /// Creates a snapshot of the lexer's current state, which can be used to
    /// restore the lexer to this position later.
    /// The snapshot can be restored by calling `Lexer::restore`.
    fn snapshot(&self) -> LexerState {
        LexerState {
            token_buffer_index: self.token_buffer_index,
            input_cursor: self.input_cursor,
            input_row: self.input_row,
            input_column: self.input_column,
            state: self.state.clone(),
        }
    }

    /// Restores the lexer's state to a previous snapshot created by `Lexer::snapshot`.
    fn restore(&mut self, state: &LexerState) -> Option<LexerError> {
        if state.token_buffer_index > self.token_buffer.len() {
            return Some(LexerError::InvalidSnapshot {
                message: format!(
                    "Invalid token buffer index: {} (buffer length: {})",
                    state.token_buffer_index,
                    self.token_buffer.len()
                ),
            });
        }
        self.token_buffer_index = state.token_buffer_index;
        self.input_cursor = state.input_cursor;
        self.input_row = state.input_row;
        self.input_column = state.input_column;
        self.state = state.state.clone();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_identifiers_and_skips_whitespace() {
        let rules = vec![
            LexerRule {
                pattern: r"[a-zA-Z_][a-zA-Z0-9_]*",
                kind: "ident",
                keep: true,
                modification: StateModification::None,
            },
            LexerRule {
                pattern: r"\s+",
                kind: "whitespace",
                keep: false,
                modification: StateModification::None,
            },
        ];

        let mut lexer = LazyStatefulLexer::new("foo bar", rules).expect("lexer should build");

        let first = lexer.next().expect("first token");
        assert_eq!(first.kind, "ident");
        assert_eq!(first.text, "foo");

        let second = lexer.next().expect("second token");
        assert_eq!(second.kind, "ident");
        assert_eq!(second.text, "bar");

        match lexer.next() {
            Err(LexerError::Eof) => {}
            other => panic!("expected Eof, got {:?}", other),
        }
    }

    #[test]
    fn chooses_longest_match() {
        let rules = vec![
            LexerRule {
                pattern: r"foobar",
                kind: "foobar",
                keep: true,
                modification: StateModification::None,
            },
            LexerRule {
                pattern: r"foo",
                kind: "foo",
                keep: true,
                modification: StateModification::None,
            },
        ];

        let mut lexer = LazyStatefulLexer::new("foobar", rules).expect("lexer should build");
        let token = lexer.next().expect("token");
        assert_eq!(token.kind, "foobar");
        assert_eq!(token.text, "foobar");
    }

    #[test]
    fn prefers_first_rule_on_tie() {
        let rules = vec![
            LexerRule {
                pattern: r"a.",
                kind: "first",
                keep: true,
                modification: StateModification::None,
            },
            LexerRule {
                pattern: r"ab",
                kind: "second",
                keep: true,
                modification: StateModification::None,
            },
        ];

        let mut lexer = LazyStatefulLexer::new("ab", rules).expect("lexer should build");
        let token = lexer.next().expect("token");
        assert_eq!(token.kind, "first");
        assert_eq!(token.text, "ab");
    }

    #[test]
    fn snapshot_and_restore_rewinds_token_stream() {
        let rules = vec![
            LexerRule {
                pattern: r"[a-z]+",
                kind: "ident",
                keep: true,
                modification: StateModification::None,
            },
            LexerRule {
                pattern: r"\s+",
                kind: "whitespace",
                keep: false,
                modification: StateModification::None,
            },
        ];

        let mut lexer = LazyStatefulLexer::new("one two", rules).expect("lexer should build");

        let first = lexer.next().expect("first token");
        assert_eq!(first.text, "one");

        let snapshot = lexer.snapshot();

        let second = lexer.next().expect("second token");
        assert_eq!(second.text, "two");

        // Restore and read again; we should see the same second token.
        assert!(lexer.restore(snapshot).is_none());
        let second_again = lexer.next().expect("second token after restore");
        assert_eq!(second_again.kind, second.kind);
        assert_eq!(second_again.text, second.text);
        assert_eq!(second_again.position.row_begin, second.position.row_begin);
        assert_eq!(
            second_again.position.column_begin,
            second.position.column_begin
        );
    }

    #[test]
    fn reports_no_match_error() {
        let rules = vec![LexerRule {
            pattern: r"[0-9]+",
            kind: "number",
            keep: true,
            modification: StateModification::None,
        }];

        let mut lexer = LazyStatefulLexer::new("abc", rules).expect("lexer should build");

        match lexer.next() {
            Err(LexerError::NoMatch) => {}
            other => panic!("expected NoMatch, got {:?}", other),
        }
    }

    #[test]
    fn computes_positions_across_newlines() {
        let rules = vec![
            LexerRule {
                pattern: r"\s+",
                kind: "whitespace",
                keep: false,
                modification: StateModification::None,
            },
            LexerRule {
                pattern: r"[a-z]+",
                kind: "ident",
                keep: true,
                modification: StateModification::None,
            },
        ];

        let mut lexer = LazyStatefulLexer::new("\nabc", rules).expect("lexer should build");

        // First token is whitespace with a newline, which is skipped.
        let token = lexer.next().expect("identifier after newline");
        assert_eq!(token.kind, "ident");
        assert_eq!(token.text, "abc");
        assert_eq!(token.position.row_begin, 1);
        assert_eq!(token.position.column_begin, 0);
        assert_eq!(token.position.row_end, 1);
        assert_eq!(token.position.column_end, 3);
    }

    static INNER_RULES: &[LexerRule] = &[LexerRule {
        pattern: r"[0-9]+",
        kind: "inner_number",
        keep: true,
        modification: StateModification::Pop,
    }];

    static ROOT_RULES: &[LexerRule] = &[
        LexerRule {
            pattern: r"\[",
            kind: "lbracket",
            keep: false,
            modification: StateModification::Push(INNER_RULES),
        },
        LexerRule {
            pattern: r"[0-9]+",
            kind: "root_number",
            keep: true,
            modification: StateModification::None,
        },
        LexerRule {
            pattern: r"\s+",
            kind: "whitespace",
            keep: false,
            modification: StateModification::None,
        },
    ];

    #[test]
    fn push_and_pop_state_changes_ruleset() {
        let mut lexer =
            LazyStatefulLexer::new("[123 456", ROOT_RULES.to_vec()).expect("lexer should build");

        // '[' pushes INNER_RULES and is discarded.
        let first = lexer.next().expect("inner number after bracket");
        assert_eq!(first.kind, "inner_number");
        assert_eq!(first.text, "123");

        // After INNER_RULES token, state is popped back to ROOT_RULES.
        let second = lexer.next().expect("root number");
        assert_eq!(second.kind, "root_number");
        assert_eq!(second.text, "456");
    }

    #[test]
    fn pop_from_single_state_produces_error() {
        let rules = vec![LexerRule {
            pattern: r"[0-9]+",
            kind: "number",
            keep: true,
            modification: StateModification::Pop,
        }];

        let mut lexer = LazyStatefulLexer::new("123", rules).expect("lexer should build");

        match lexer.next() {
            Err(LexerError::InvalidState { .. }) => {}
            other => panic!("expected InvalidState, got {:?}", other),
        }
    }

    #[test]
    fn invalid_snapshot_is_reported() {
        let rules = vec![LexerRule {
            pattern: r"[a-z]+",
            kind: "ident",
            keep: true,
            modification: StateModification::None,
        }];

        let mut lexer = LazyStatefulLexer::new("one", rules).expect("lexer should build");

        let mut snapshot = lexer.snapshot();
        // Corrupt the snapshot so that the token_buffer_index is out of range.
        snapshot.token_buffer_index = 10;

        match lexer.restore(snapshot) {
            Some(LexerError::InvalidSnapshot { .. }) => {}
            other => panic!("expected InvalidSnapshot, got {:?}", other),
        }
    }
}
