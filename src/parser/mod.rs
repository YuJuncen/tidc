use std::{error::{self}, fmt::{self, Display}};

pub mod artifacts;
mod scanner;

#[derive(Debug)]
pub enum ParseError {
    Unexpected {
        expected: String,
        got: String,
        hint: String,
    },
    Empty
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
