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
pub enum StateModification<'a> {
    /// The lexer's stack remains unchanged.
    None,
    /// The top ruleset is popped from the lexer's stack. If the stack only
    /// contains one ruleset, this results in a `LexerError::InvalidState`.
    Pop,
    /// Pushes a new ruleset onto the lexer's stack.
    Push(&'a [LexerRule<'a>]),
    /// Similar to `Push`, but the ruleset is not provided directly. Instead, a
    /// factory function is provided that returns the ruleset when called. This
    /// allows for defining cyclic rulesets, which is not possible with the
    /// standard `Push` variant.
    PushLazy(&'a (dyn Fn() -> &'a [LexerRule<'a>] + Send + Sync)),
}

/// A lexer rule defines which tokens are generated from the input. They also
/// control the state of the lexer by allowing rulesets (states) to be pushed
/// and popped from the lexer's internal stack.
#[derive(Clone)]
pub struct LexerRule<'a> {
    pub pattern: &'static str,
    pub kind: &'static str,
    pub keep: bool,
    pub modification: StateModification<'a>,
}

/// After a ruleset of the lexer is compiled, it is stored in a shared arena.
/// The `RulesetIndex` is a numeric index that is used to refer to the compiled
/// ruleset in the arena.
type RulesetIndex = usize;

/// The compiled version of the state modification does not include a `PushLazy`
/// variant because cyclic references, which are implemented using `PushLazy`,
/// are resolved during compilation.
#[derive(Clone)]
enum CompiledStateModification {
    None,
    Pop,
    Push { index: RulesetIndex },
}

/// The lexer compiles regular `LexerRule`s into `CompiledLexerRule`s, which
/// contain compiled regex patterns, for instance. The regular `LexerRule`s can
/// be defined statically, while the compiled variants cannot. The compiled
/// variants are stored in the lexer during runtime and used for tokenization.
#[derive(Clone)]
struct CompiledLexerRule {
    pattern: Regex,
    kind: &'static str,
    keep: bool,
    modification: CompiledStateModification,
}

#[derive(Debug)]
pub enum LexerError {
    NoMatch,
    Eof,
    InvalidSnapshot { message: String },
    InvalidState { message: String },
    InvalidRule { message: String },
    Custom { message: String },
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
pub struct LexerState {
    token_buffer_index: usize,
    input_cursor: usize,
    input_row: usize,
    input_column: usize,
}

// A pull-based lexer that allows its caller to save and restore its state
// relative to the input sequence.
pub trait Lexer<'a> {
    fn next(&mut self) -> Result<Token<'a>, LexerError>;
    fn snapshot(&self) -> LexerState;
    fn restore(&mut self, state: LexerState) -> Option<LexerError>;
}

/// A collection of compiled lexer rules. To match a token, the lexer only
/// ever looks at a single ruleset, which is the topmost one on the internal stack.
type LexerRuleset = Vec<CompiledLexerRule>;

/// A key that uniquely identifies a ruleset based on the pointer and length of
/// the rules slice. This is used to keep track of already compiled rulesets and
/// their indices, so that recursive and cyclic rulesets are compiled without
/// getting into infinite loops.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct RulesetKey {
    ptr: *const (),
    len: usize,
}

impl RulesetKey {
    fn from_rules(rules: &[LexerRule]) -> Self {
        RulesetKey {
            ptr: rules.as_ptr() as *const (),
            len: rules.len(),
        }
    }
}

/// Used to keep track of already compiled rulesets and their indices, so that
/// recursive and cyclic rulesets are compiled without getting into infinite loops.
struct CompileContext {
    rulesets: Vec<LexerRuleset>,
    indices: HashMap<RulesetKey, usize>,
}

/// Compiles a ruleset and all of its nested rulesets into `CompiledLexerRule`s
/// and stores them in the `CompileContext`. If the same ruleset is encountered
/// repeatedly, the previously assigned index is returned, which
/// allows for recursive and cyclic rulesets to be compiled creating an infinite
/// recursion.
fn compile_ruleset<'s, 'a>(
    rules: &'s [LexerRule<'a>],
    context: &mut CompileContext,
) -> Result<RulesetIndex, LexerError> {
    let key = RulesetKey::from_rules(rules);
    // Check whether the ruleset is currently or has
    // already been compiled and return its index if so.
    if let Some(&index) = context.indices.get(&key) {
        return Ok(index);
    }
    let index = context.rulesets.len();
    context.indices.insert(key, index);
    // Insert a placeholder to allow self/cyclic references
    context.rulesets.push(Vec::new());
    let compiled_rules = rules
        .iter()
        .map(|rule| {
            let pattern = Regex::new(rule.pattern)?;
            let modification = match rule.modification {
                StateModification::None => CompiledStateModification::None,
                StateModification::Pop => CompiledStateModification::Pop,
                StateModification::Push(nested) => {
                    let target = compile_ruleset(nested, context)?;
                    CompiledStateModification::Push { index: target }
                }
                StateModification::PushLazy(factory) => {
                    let nested = factory();
                    let target = compile_ruleset(nested, context)?;
                    CompiledStateModification::Push { index: target }
                }
            };
            Ok(CompiledLexerRule {
                pattern,
                kind: rule.kind,
                keep: rule.keep,
                modification,
            })
        })
        .collect::<Result<LexerRuleset, LexerError>>()?;
    context.rulesets[index] = compiled_rules;
    Ok(index)
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
    rulesets: Vec<LexerRuleset>,
    state: Vec<RulesetIndex>,
    token_buffer: Vec<Token<'input>>,
    token_buffer_index: usize,
    input_cursor: usize,
    input_row: usize,
    input_column: usize,
}

impl<'input> LazyStatefulLexer<'input> {
    pub fn new(input: &'input str, rules: Vec<LexerRule>) -> Result<Self, LexerError> {
        let mut context = CompileContext {
            rulesets: Vec::new(),
            indices: HashMap::new(),
        };
        let root_index = compile_ruleset(&rules[..], &mut context)?;
        let state = vec![root_index];
        Ok(Self {
            input,
            rulesets: context.rulesets,
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
            let token = &self.token_buffer[self.token_buffer_index];
            self.token_buffer_index += 1;
            Ok(token.clone())
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
            let mut best_modification: &CompiledStateModification =
                &CompiledStateModification::None;
            let current_ruleset_index = match self.state.last() {
                Some(index) => *index,
                None => {
                    return Err(LexerError::InvalidState {
                        message: String::from(
                            "The lexer is currently not equipped with any rulesets.",
                        ),
                    });
                }
            };
            let current_ruleset = match self.rulesets.get(current_ruleset_index) {
                Some(ruleset) => ruleset,
                None => {
                    return Err(LexerError::InvalidState {
                        message: format!(
                            "The lexer refers to an unknown ruleset index: {}",
                            current_ruleset_index
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
                CompiledStateModification::None => {}
                CompiledStateModification::Pop => {
                    if self.state.len() <= 1 {
                        return Err(LexerError::InvalidState {
                            message: String::from(
                                "The lexer state cannot be popped because it only contains one ruleset.",
                            ),
                        });
                    }
                    self.state.pop();
                }
                CompiledStateModification::Push {
                    index: target_index,
                } => {
                    self.state.push(*target_index);
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
            self.token_buffer.push(new_token.clone());
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
        }
    }

    /// Restores the lexer's state to a previous snapshot created by `Lexer::snapshot`.
    fn restore(&mut self, state: LexerState) -> Option<LexerError> {
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
