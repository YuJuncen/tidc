use std::{str::FromStr};

use super::{Error, Scanner};

#[derive(Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown
}

#[derive(Debug, PartialEq, Eq)]
pub struct LogFieldRef<'a> {
    pub key: LogStr<'a>,
    pub value: LogStr<'a>,
}

/// LogRecordRef is a line of PingCAP log.
#[derive(Debug)]
pub struct LogRecordRef<'a> {
    pub level: LogLevel,
    pub time: &'a str,
    pub message: LogStr<'a>,
    pub source: Option<FileLineRef<'a>>,
    pub entries: Vec<LogFieldRef<'a>>
}

#[derive(Debug)]
pub struct FileLineRef<'a> {
    pub file: &'a str,
    pub line: &'a str,
}

#[derive(Debug)]
pub struct TimeRef<'a> {
    pub time_str: &'a str
}

#[derive(Debug, Eq, PartialEq)]
pub enum LogStr<'a> {
    Quoted(&'a str),
    Unquoted(&'a str)
}

impl <'a> LogStr<'a> {
    fn from_str(s: &'a str) -> Result<Self, Error> {
        match s.chars().next() {
            None => Ok(Self::Unquoted("")),
            // TODO: check whether the string is rightly ends with '"'
            Some('"') => Ok(Self::Quoted(s)),
            Some(_) => Ok(Self::Unquoted(s))
        }
    }

    fn parse_from_sequence(text: &Scanner<'a>) -> Result<Self, Error> {
        let got = match text.peek_char() {
            Some('"') => text.quoted_string(),
            Some(_) => text.unquoted_string(),
            None => Err(super::empty()),
        }?;
        Self::from_str(got)
    }
}


impl<'a> FileLineRef<'a> {
    const UNKNOWN : &'static str = "<unknown>";

    fn from_str(s: &'a str) -> Option<Self> {
        if s == Self::UNKNOWN {
            return None
        }

        let mut colons = Vec::with_capacity(8);
        for (i, ch) in s.char_indices() {
            if ch == ':' {
                colons.push(i);
            }
        }
        colons.last().map(|i| {
            let (file, line) = s.split_at(*i);
            Self { file, line: &line[1..] }
        })
    }

    fn parse_from_str<'b: 'a>(text: &'b Scanner<'a>) -> Result<Option<Self>, Error> {
        let source = text.in_bracket( |s: &Scanner| { s.till_next_bracket() })?;
        Ok(Self::from_str(source))
    }
}

impl<'a> LogFieldRef<'a> {
    fn parse_from_str(text: &'a Scanner) -> Result<Self, Error> {
        let key = LogStr::parse_from_sequence(text)?;
        text.consume_exact('=')?;

        let value = LogStr::parse_from_sequence(text)?;

        Ok(Self { key, value })
    }

    fn parse_from_field<'b : 'a>(text: &'b Scanner<'a>) -> Result<Self, Error> {
        text.in_bracket(Self::parse_from_str)
    }
}

impl LogLevel {
    fn parse_from_str<'a, 'b:'a>(text: &'b Scanner<'a>) -> Result<Self, Error> {
        let field = text.in_bracket(|s| s.till_next_bracket())?;
        Self::from_str(field)
    }
}

impl FromStr for LogLevel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s {
            "FATAL" => Self::Fatal,
            "ERROR" => Self::Error,
            "WARN" => Self::Warn,
            "INFO" => Self::Info,
            "DEBUG" => Self::Debug,
            _ => Self::Unknown,
        };
        Ok(result)
    }
}

impl<'a> LogRecordRef<'a> {
    fn parse_from_str<'b: 'a>(scanner: &'b Scanner<'a>) -> Result<Self, Error> {
        let time = scanner.in_bracket(|s| s.till_next_bracket())?;
        scanner.skip_space();
        let level= LogLevel::parse_from_str(&scanner)?;
        scanner.skip_space();
        let source = FileLineRef::parse_from_str(&scanner)?;
        scanner.skip_space();
        let message = scanner.in_bracket(LogStr::parse_from_sequence)?;
        scanner.skip_space();

        let mut entries = Vec::with_capacity(16);
        while !scanner.is_done() {
            let field = match LogFieldRef::parse_from_field(& scanner) {
                Err(err) => {
                    // TODO use slog!
                    eprintln!("meet error {} during parsing, skipping this field (log = {})", err, scanner.target.get());
                    scanner.skip_until(|c| c == ']');
                    scanner.consume_exact(']')?;
                    continue;
                }
                Ok(pair) => pair
            };
            
            entries.push(field);
            scanner.skip_space()
        }
        Ok(Self { level, time, source, message, entries })
    }
}

pub fn with_log_record<'a, T: 'a>(s: &'a str, callback: impl FnOnce(LogRecordRef<'_>) -> T) -> Result<T, Error> {
    let scanner = Scanner::over(s);
    Ok(callback(LogRecordRef::parse_from_str(&scanner)?))
}

mod displaying {
    use std::fmt::{self, Display};

    use super::LogStr;

    impl <'a> Display for LogStr<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Quoted(s) => f.write_str(s),
                Self::Unquoted(s) => {
                    f.write_str("\"")?;
                    f.write_str(s)?;
                    f.write_str("\"")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Scanner;

    #[test]
    fn test_log_str() {
        use super::LogStr;

        fn check(from: &str, to: &str) {
            let log_str = LogStr::from_str(from);
            assert!(log_str.is_ok());
            let log_str = log_str.unwrap();
            assert_eq!(format!("{}", log_str), to);
        }

    
        check("42µs", r#""42µs""#);
        check(r#""hello, world""#, r#""hello, world""#);
        check("42µs", r#""42µs""#);

        fn check_scan(from: &str, to: &str, rem: &str) {
            let scanner = Scanner::over(from);
            let log_str = LogStr::parse_from_sequence(&scanner);
            assert!(log_str.is_ok());
            let log_str = log_str.unwrap();
            assert_eq!(format!("{}", log_str), to);
            assert_eq!(scanner.remain(), rem)
        }

        check_scan("date-time=12:15:13", "\"date-time\"", "=12:15:13");
        check_scan(r#""rate limit"=128MB/s"#, "\"rate limit\"", "=128MB/s");
    }

    #[test]
    fn test_log_entry() {
        use super::LogFieldRef;

        fn check(from: &str, key: &str, value: &str) {
            let scanner = Scanner::over(from);
            let entry = LogFieldRef::parse_from_str(&scanner);
            assert!(entry.is_ok(), "parsing entry meet error {}", entry.unwrap_err());

            let entry = entry.unwrap();
            assert_eq!(format!("{}", entry.key), key, "failed to check {} (key mismatch {} => {})", from, entry.key, key);
            assert_eq!(format!("{}", entry.value), value, "failed to check {} (value mismatch {} => {})", from, entry.value, value);
        }

        check(r#""start time"=992.547µs""#, r#""start time""#, r#""992.547µs""#);
        check("date-time=12:15:13", r#""date-time""#, r#""12:15:13""#);
        check(r#""rate limit"=128MB/s"#, r#""rate limit""#, r#""128MB/s""#);
        let entry = r#""rate l\n\"imit"="128 MB/s""#.to_owned();
        check(&entry, r#""rate l\n\"imit""#, r#""128 MB/s""#);
    }
}
