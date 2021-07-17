use std::{cell::Cell};

use super::{ParseError};

pub struct Scanner<'a> {
    target: &'a str,
    remain: Cell<&'a str>,
    offset: Cell<usize>
}

impl<'a> Scanner<'a> {
    pub fn over(s: &'a str) -> Self {
        Scanner {
            target: s,
            remain: Cell::new(s),
            offset: Cell::new(0)
        }
    }

    pub fn is_done(&self) -> bool {
        self.offset.get() >= self.target.len()
    }

    pub fn remain(&self) -> &'a str {
        return self.remain.get()
    }

    pub fn consume(&self, n: usize) -> Result<&'a str, ParseError> {
        let new_offset = self.offset.get() + n;
        if new_offset > self.target.len() {
            return Err(ParseError::Empty)
        }
        self.offset.set(self.offset.get() + n);
        let (consumed, remain) = self.remain.get().split_at(n);
        self.remain.set(remain);
        return Ok(consumed)
    }

    pub fn drain(&self) -> Result<&'a str, ParseError> {
        return self.consume(self.target.len() - self.offset.get())
    } 

    pub fn peek_char(&self) -> Option<char> {
        return self.remain().chars().next()
    }

    pub fn unquoted_string(&self) -> Result<&'a str, ParseError> {
        for (i, ch) in self.remain().char_indices() {
            if char_need_quote(ch) {
                return Ok(self.consume(i)?);
            }
        }
        return Ok(self.drain()?)
    }

    pub fn current_char(&self) -> char {
        self.remain().chars().next().unwrap_or('$')
    }

    pub fn unexpected(&self, expected: impl ToString, got: impl ToString) -> ParseError {
        ParseError::Unexpected {
            expected: expected.to_string(),
            got: got.to_string(),
            hint: format!("{}>{}<{}", self.context_before(), self.current_char(), self.context_after())
        }
    }

    pub fn context_before(&self) -> &str {
        let amount = 5usize;
        match self.offset.get() {
            0 => "^",
            x if x <= amount => &self.target[..self.offset.get()],
            _ => &self.target[self.offset.get()-amount..self.offset.get()]
        }
    }

    pub fn context_after(&self) -> &str {
        let amount = 10usize;
        match self.target.len() - self.offset.get() {
            0 => "",
            x if x <= amount => &self.remain()[..x],
            _ => &self.remain()[..amount]
        }
    }

    pub fn quoted_string(&self) -> Result<&'a str, ParseError> {
        enum State {
            Escaping,
            Scanning
        }
        let mut state = State::Escaping;
        for (i, ch) in self.remain().char_indices() {
            match state {
                State::Escaping => { state = State::Scanning; }
                State::Scanning => {
                    match ch {
                        // split at i + 1 to include the close '"' char.
                        '"' => return Ok(self.consume(i+1)?),
                        '\\' => state = State::Escaping,
                        _ => continue
                    };
                }
            } 
        }
        Err(self.unexpected("\"", "EOF"))
    }

    pub fn in_bracket<'this, T>(&'this self, inner_parser: impl FnOnce(&'this Self)->Result<T, ParseError>) -> Result<T, ParseError> {
        self.consume_exact('[')?;
        let result = inner_parser(self)?;
        self.consume_exact(']')?;
        Ok(result)
    }

    pub fn assert_current_is(&self, expected: char) -> Result<(), ParseError> {
        match self.remain().chars().next() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(self.unexpected(expected, ch)),
            None => Err(empty())
        }
    }

    pub fn consume_exact(&self, expected: char) -> Result<(), ParseError> {
        self.assert_current_is(expected)?;
        self.consume(1)?;
        Ok(())
    }

    pub fn skip_until(& self, f: impl Fn(char) -> bool) {
        for (i, ch) in self.remain().char_indices() {
            if f(ch) {
                match self.consume(i) {
                    Ok(_) => { return }
                    _ => unreachable!()
                }
            }
        }
    }

    pub fn skip_while(& self, f: impl Fn(char) -> bool) {
        self.skip_until(|x| !f(x))
    }

    pub fn skip_space(& self) {
        self.skip_while(char::is_whitespace)
    }

    pub fn till_next_bracket(& self) -> Result<&str, ParseError> {
        for (i, ch) in self.remain().char_indices() {
            if ch == ']' {
                return Ok(self.consume(i)?)
            }
        }
        Err(empty())
    }
}

fn char_need_quote(ch: char) -> bool {
    match ch {
        '\x00'..='\x20' | '=' | '"' | '[' | ']'  => true,
        _ => false,
    }
}

pub fn empty() -> ParseError {
    ParseError::Empty
}