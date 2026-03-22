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
pub struct LexerRule {
    pub pattern: &'static str,
    pub kind: &'static str,
}

#[derive(Clone, Copy)]
pub enum StateModification<'a> {
    None,
    Pop,
    Push(&'a [LexerDirective<'a>]),
    PushLazy(&'a (dyn Fn() -> &'a [LexerDirective<'a>] + Send + Sync)),
}

#[derive(Clone, Copy)]
pub struct LexerDirective<'a> {
    pub rule: LexerRule,
    pub keep: bool,
    pub modification: StateModification<'a>,
}

struct CompiledLexerDirective<'a> {
    pattern: Regex,
    kind: &'static str,
    keep: bool,
    modification: StateModification<'a>,
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
}

// A pull-based lexer that allows its caller to save and restore its state relative to the input sequence.
pub trait Lexer<'a> {
    fn next(&mut self) -> Result<Token<'a>, LexerError>;
    fn snapshot(&self) -> LexerState;
    fn restore(&mut self, state: LexerState) -> Option<LexerError>;
}

/// A lexer that takes an input string and a set of rules and produces a stream of tokens.
/// The lexer is pull-based and lazy, meaning that it only produces tokens when requested
/// and only processes the minimum amount of input necessary to produce the next token.
/// The lexer maintains an internal buffer of tokens that have been matched so far, so that if the lexer is
/// reset to a previous position, the tokens can be returned from the buffer without having to re-match
/// the input.
pub struct LazyLexer<'input, 'rules> {
    input: &'input str,
    rules: Vec<CompiledLexerDirective<'rules>>,
    token_buffer: Vec<Token<'input>>,
    token_buffer_index: usize,
    input_cursor: usize,
    input_row: usize,
    input_column: usize,
}

impl<'input, 'rules> LazyLexer<'input, 'rules> {
    pub fn new(input: &'input str, rules: Vec<LexerDirective<'rules>>) -> Self {
        let compiled_rules = rules
            .into_iter()
            .map(|LexerDirective { rule, keep, modification }| CompiledLexerDirective {
                // TODO: translate to a `LexerError`
                pattern: Regex::new(rule.pattern).expect("invalid lexer regex"),
                kind: rule.kind,
                keep,
                modification,
            })
            .collect::<Vec<_>>();
        Self {
            input,
            rules: compiled_rules,
            token_buffer: Vec::new(),
            token_buffer_index: 0,
            input_cursor: 0,
            input_row: 0,
            input_column: 0,
        }
    }
}

impl<'input, 'rules> Lexer<'input> for LazyLexer<'input, 'rules> {
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
            for CompiledLexerDirective {
                pattern,
                kind,
                keep,
                modification,
            } in self.rules.iter()
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
