use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MError {
    #[error("{0}")]
    InvalidState(String),
    #[error("Unterminated token")]
    UnterminatedToken,
    #[error("End of string reached")]
    EndOfString,
}

pub struct Lexer<'a> {
    str: &'a str,
    history: Vec<usize>,
}

enum LexerState {
    Out,
    In(String, Option<char>),
}

impl<'a> Lexer<'a> {
    pub fn new(str: &'a str) -> Self {
        Self {
            str,
            history: vec![0],
        }
    }

    pub fn reset(&mut self) {
        self.history = vec![0];
    }

    pub fn rewind(&mut self, n: usize) {
        let new_len = self.history.len().saturating_sub(n);
        self.history
            .truncate(if new_len != 0 { new_len } else { 1 });
    }

    pub fn get_token(&mut self) -> Result<String, MError> {
        let start = *self
            .history
            .last()
            .ok_or_else(|| MError::InvalidState("Empty history".to_owned()))?;
        let rest = &self.str[start..];

        let mut it = rest.char_indices();
        let mut state = LexerState::Out;

        loop {
            let Some((i, c)) = it.next() else {
                match state {
                    LexerState::Out => return Err(MError::EndOfString),
                    LexerState::In(_, Some(_)) => return Err(MError::UnterminatedToken),
                    LexerState::In(c_token, None) => {
                        self.history.push(self.str.len());
                        return Ok(c_token);
                    }
                }
            };

            match state {
                LexerState::Out => {
                    if !c.is_whitespace() {
                        if c == '\'' || c == '"' {
                            state = LexerState::In(String::new(), Some(c));
                        } else {
                            state = LexerState::In(c.to_string(), None);
                        }
                    }

                    continue;
                }
                LexerState::In(ref mut c_token, None) => {
                    if c.is_whitespace() {
                        self.history.push(start + i);
                        return Ok(std::mem::take(c_token));
                    }

                    if c == '\'' || c == '"' {
                        state = LexerState::In(std::mem::take(c_token), Some(c));
                        continue;
                    }

                    c_token.push(c);
                }
                LexerState::In(ref mut c_token, Some(quote_type)) => {
                    if c == quote_type {
                        state = LexerState::In(std::mem::take(c_token), None);
                        continue;
                    }

                    c_token.push(c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_get_token() {
        // ["hello", "world", "a\"a\"aa", "a'b'bb", "a a", "b b"]
        let mut lexer = Lexer::new("hello w\"o\"rl'd' a'\"a\"a'a a\"'b'b\"b \"a a\" 'b b'");
        assert_eq!(lexer.get_token().as_deref(), Ok("hello"));
        assert_eq!(lexer.get_token().as_deref(), Ok("world"));
        assert_eq!(lexer.get_token().as_deref(), Ok("a\"a\"aa"));
        assert_eq!(lexer.get_token().as_deref(), Ok("a'b'bb"));
        assert_eq!(lexer.get_token().as_deref(), Ok("a a"));
        assert_eq!(lexer.get_token().as_deref(), Ok("b b"));
        assert_eq!(lexer.get_token(), Err(MError::EndOfString));
    }

    #[test]
    fn t_unterminated_tokens() {
        let mut lexer = Lexer::new("'hell");
        assert_eq!(lexer.get_token(), Err(MError::UnterminatedToken));

        lexer = Lexer::new("\"foo");
        assert_eq!(lexer.get_token(), Err(MError::UnterminatedToken));

        lexer.history = vec![];
        assert!(matches!(lexer.get_token(), Err(MError::InvalidState(_))));
    }

    #[test]
    fn t_rewind() {
        let mut lexer = Lexer::new("Lorem ipsum dolor sit amet");
        assert_eq!(lexer.get_token().as_deref(), Ok("Lorem"));
        assert_eq!(lexer.get_token().as_deref(), Ok("ipsum"));
        assert_eq!(lexer.get_token().as_deref(), Ok("dolor"));

        lexer.rewind(1);
        assert_eq!(lexer.get_token().as_deref(), Ok("dolor"));

        lexer.rewind(2);
        assert_eq!(lexer.get_token().as_deref(), Ok("ipsum"));
        assert_eq!(lexer.get_token().as_deref(), Ok("dolor"));

        lexer.rewind(10);
        assert_eq!(lexer.get_token().as_deref(), Ok("Lorem"));
    }
}
