use std::sync::Arc;

use regex::Regex;

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

#[derive(Clone, Copy)]
pub enum StateModification<'a> {
    None,
    Pop,
    Push(&'a [LexerRule<'a>]),
    PushLazy(&'a (dyn Fn() -> &'a [LexerRule<'a>] + Send + Sync)),
}

#[derive(Clone)]
pub struct LexerRule<'a> {
    pub pattern: &'static str,
    pub kind: &'static str,
    pub keep: bool,
    pub modification: StateModification<'a>,
}

#[derive(Clone)]
pub enum CompiledStateModification<'a> {
    None,
    Pop,
    Push(Vec<CompiledLexerRule<'a>>),
    PushLazy(Arc<dyn Fn() -> Vec<CompiledLexerRule<'a>> + Send + Sync + 'a>),
}

impl<'a> From<StateModification<'a>> for CompiledStateModification<'a> {
    fn from(modification: StateModification<'a>) -> Self {
        match modification {
            StateModification::None => CompiledStateModification::None,
            StateModification::Pop => CompiledStateModification::Pop,
            StateModification::Push(rules) => {
                let compiled_rules = rules.iter().cloned().map(CompiledLexerRule::from).collect();
                CompiledStateModification::Push(compiled_rules)
            }
            StateModification::PushLazy(factory) => {
                let compiled_factory: Arc<
                    dyn Fn() -> Vec<CompiledLexerRule<'a>> + Send + Sync + 'a,
                > = Arc::new(move || {
                    let rules = factory();
                    rules.iter().cloned().map(CompiledLexerRule::from).collect()
                });
                CompiledStateModification::PushLazy(compiled_factory)
            }
        }
    }
}

#[derive(Clone)]
struct CompiledLexerRule<'a> {
    pattern: Regex,
    kind: &'static str,
    keep: bool,
    modification: CompiledStateModification<'a>,
}

impl<'a> From<LexerRule<'a>> for CompiledLexerRule<'a> {
    fn from(rule: LexerRule<'a>) -> Self {
        Self {
            pattern: Regex::new(rule.pattern).unwrap(),
            kind: rule.kind,
            keep: rule.keep,
            modification: rule.modification.into(),
        }
    }
}

/// A snapshot of the lexer's state, which can be used to restore the lexer to a previous position.
/// The snapshot can be created using `Lexer::snapshot` and restored using `Lexer::restore`.
pub struct LexerState {
    token_buffer_index: usize,
    input_cursor: usize,
    input_row: usize,
    input_column: usize,
}

pub enum LexerError {
    NoMatch,
    Eof,
    InvalidSnapshot { message: String },
    InvalidState { message: String },
}

// A pull-based lexer that allows its caller to save and restore its state relative to the input sequence.
pub trait Lexer<'a> {
    fn next(&mut self) -> Result<Token<'a>, LexerError>;
    fn snapshot(&self) -> LexerState;
    fn restore(&mut self, state: LexerState) -> Option<LexerError>;
}

type LexerRuleset<'a> = Vec<CompiledLexerRule<'a>>;

/// A lexer that takes an input string and a set of rules and produces a stream of tokens.
/// The lexer is pull-based and lazy, meaning that it only produces tokens when requested
/// and only processes the minimum amount of input necessary to produce the next token.
/// The lexer maintains an internal buffer of tokens that have been matched so far, so that if the lexer is
/// reset to a previous position, the tokens can be returned from the buffer without having to re-match
/// the input.
pub struct LazyStatefulLexer<'input, 'rules> {
    input: &'input str,
    state: Vec<LexerRuleset<'rules>>,
    token_buffer: Vec<Token<'input>>,
    token_buffer_index: usize,
    input_cursor: usize,
    input_row: usize,
    input_column: usize,
}

impl<'input, 'rules> LazyStatefulLexer<'input, 'rules> {
    pub fn new(input: &'input str, rules: Vec<LexerRule<'rules>>) -> Self {
        let compiled_rules = rules.into_iter().map(CompiledLexerRule::from).collect();
        let initial_state = vec![compiled_rules];
        Self {
            input,
            state: initial_state,
            token_buffer: Vec::new(),
            token_buffer_index: 0,
            input_cursor: 0,
            input_row: 0,
            input_column: 0,
        }
    }
}

impl<'input, 'rules> Lexer<'input> for LazyStatefulLexer<'input, 'rules> {
    /// Returns the next token from the input. If there are no more tokens, `LexerError::Eof` is returned.
    /// If the next token cannot be matched by any of the rules, `LexerError::NoMatch` is returned.
    /// For each token, the longest match is chosen. If there are multiple matches
    /// of the same length, the one defined first in the rules is chosen.
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
            let current_ruleset = match self.state.last() {
                Some(ruleset) => ruleset,
                None => {
                    return Err(LexerError::InvalidState {
                        message: String::from(
                            "The lexer is currently not equipped with any rulesets.",
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
                CompiledStateModification::Push(new_rules) => {
                    self.state.push(new_rules.clone());
                }
                CompiledStateModification::PushLazy(factory) => {
                    let new_rules = factory();
                    self.state.push(new_rules);
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

    /// Creates a snapshot of the lexer's current state, which can be used to restore the lexer to this position later.
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
