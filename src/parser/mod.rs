use std::{cell::Cell, error::{self}, fmt::{self, Display}, usize};

pub mod artifacts;

type Error = Box<dyn error::Error>;

#[derive(Debug)]
enum ParseError {
    Unexpected {
        expected: String,
        got: String,
        hint: String,
    },
    Empty
}

struct Scanner<'a> {
    target: Cell<&'a str>,
    remain: Cell<&'a str>,
    offset: Cell<usize>
}

impl<'a> Scanner<'a> {
    fn over(s: &'a str) -> Self {
        Scanner {
            target: Cell::new(s),
            remain: Cell::new(s),
            offset: Cell::new(0)
        }
    }

    fn is_done(&self) -> bool {
        self.offset.get() >= self.target.get().len()
    }

    fn remain(&self) -> &'a str {
        return self.remain.get()
    }

    fn consume(&self, n: usize) -> Result<&'a str, ParseError> {
        let new_offset = self.offset.get() + n;
        if new_offset > self.target.get().len() {
            return Err(ParseError::Empty)
        }
        self.offset.set(self.offset.get() + n);
        let (consumed, remain) = self.remain.get().split_at(n);
        self.remain.set(remain);
        return Ok(consumed)
    }

    fn drain(&self) -> Result<&'a str, ParseError> {
        return self.consume(self.target.get().len() - self.offset.get())
    } 

    fn peek_char(&self) -> Option<char> {
        return self.remain().chars().next()
    }

    fn unquoted_string(&self) -> Result<&'a str, Error> {
        for (i, ch) in self.remain().char_indices() {
            if char_need_quote(ch) {
                return Ok(self.consume(i)?);
            }
        }
        return Ok(self.drain()?)
    }

    fn current_char(&self) -> char {
        self.remain().chars().next().unwrap_or('$')
    }

    fn unexpected(&self, expected: impl ToString, got: impl ToString) -> Error {
        Box::new(ParseError::Unexpected {
            expected: expected.to_string(),
            got: got.to_string(),
            hint: format!("{}>{}<{}", self.context_before(), self.current_char(), self.context_after())
        })
    }

    fn context_before(&self) -> &str {
        let amount = 5usize;
        match self.offset.get() {
            0 => "^",
            x if x <= amount => &self.target.get()[..self.offset.get()],
            _ => &self.target.get()[self.offset.get()-amount..self.offset.get()]
        }
    }

    fn context_after(&self) -> &str {
        let amount = 10usize;
        match self.target.get().len() - self.offset.get() {
            0 => "",
            x if x <= amount => &self.remain()[..x],
            _ => &self.remain()[..amount]
        }
    }

    fn quoted_string(&self) -> Result<&'a str, Error> {
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

    fn in_bracket<'this, T>(&'this self, inner_parser: impl FnOnce(&'this Self)->Result<T, Error>) -> Result<T, Error> {
        self.consume_exact('[')?;
        let result = inner_parser(self)?;
        self.consume_exact(']')?;
        Ok(result)
    }

    fn assert_current_is(&self, expected: char) -> Result<(), Error> {
        match self.remain().chars().next() {
            Some(ch) if ch == expected => Ok(()),
            Some(ch) => Err(self.unexpected(expected, ch)),
            None => Err(empty())
        }
    }

    fn consume_exact(&self, expected: char) -> Result<(), Error> {
        self.assert_current_is(expected)?;
        self.consume(1)?;
        Ok(())
    }

    fn skip_until(& self, f: impl Fn(char) -> bool) {
        for (i, ch) in self.remain().char_indices() {
            if f(ch) {
                match self.consume(i) {
                    Ok(_) => { return }
                    _ => unreachable!()
                }
            }
        }
    }

    fn skip_while(& self, f: impl Fn(char) -> bool) {
        self.skip_until(|x| !f(x))
    }

    fn skip_space(& self) {
        self.skip_while(char::is_whitespace)
    }

    fn till_next_bracket(& self) -> Result<&str, Error> {
        for (i, ch) in self.remain().char_indices() {
            if ch == ']' {
                return Ok(self.consume(i)?)
            }
        }
        Err(empty())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Unexpected { expected, got, hint } => {
                write!(f, "unexpected {}, excepting {} (hint: `{}`)", got, expected, hint)?;
            }
            ParseError::Empty => { f.write_str("got empty string to parse")?; }
        }
        Ok(())
    }
}

impl error::Error for ParseError {}

fn char_need_quote(ch: char) -> bool {
    match ch {
        '\x00'..='\x20' | '=' | '"' | '[' | ']'  => true,
        _ => false,
    }
}


fn empty() -> Box<dyn error::Error> {
    Box::new(ParseError::Empty)
}

