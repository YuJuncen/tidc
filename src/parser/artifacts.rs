use std::{str::FromStr};

use tinyvec::{TinyVec};

use super::{ParseError, scanner::Scanner};

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

const TINY_VEC_THRESHOLD : usize = 12;

/// LogRecordRef is a line of PingCAP log.
#[derive(Debug)]
pub struct LogRecordRef<'a> {
    pub level: LogLevel,
    pub time: TimeRef<'a>,
    pub message: LogStr<'a>,
    pub source: Option<FileLineRef<'a>>,
    pub entries: TinyVec<[LogFieldRef<'a>; TINY_VEC_THRESHOLD]>
}

#[derive(Debug)]
pub struct FileLineRef<'a> {
    pub file: &'a str,
    pub line: &'a str,
}

#[derive(Debug)]
pub struct TimeRef<'a> {
    pub time_str: &'a str,
}

impl<'a> TimeRef<'a> {
    fn from_str_unchecked(s: &'a str) -> Self {
        Self {
            time_str: s
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum LogStr<'a> {
    Quoted(&'a str),
    Unquoted(&'a str)
}

impl <'a> LogStr<'a> {
    fn from_str(s: &'a str) -> Result<Self, ParseError> {
        match s.chars().next() {
            None => Ok(Self::Unquoted("")),
            // TODO: check whether the string is rightly ends with '"'
            Some('"') => Ok(Self::Quoted(s)),
            Some(_) => Ok(Self::Unquoted(s))
        }
    }

    fn parse_from_sequence(text: &Scanner<'a>) -> Result<Self, ParseError> {
        let got = match text.peek_char() {
            Some('"') => text.quoted_string(),
            Some(_) => text.unquoted_string(),
            None => Err(super::scanner::empty()),
        }?;
        Self::from_str(got)
    }

    fn scan_from_with_need_quote(text: &Scanner<'a>, f: impl FnMut(char) -> bool) -> Result<Self, ParseError> {
        let got = match text.peek_char() {
            Some('"') => text.quoted_string(),
            Some(_) => text.consume_until(f),
            None => Err(super::scanner::empty()),
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

    fn scan_from<'b: 'a>(text: &'b Scanner<'a>) -> Result<Option<Self>, ParseError> {
        let source = text.in_bracket( |s: &Scanner| { s.till_next_bracket() })?;
        Ok(Self::from_str(source))
    }
}

impl<'a> LogFieldRef<'a> {
    fn scan_from(text: &'a Scanner) -> Result<Self, ParseError> {
        let key = LogStr::parse_from_sequence(text)?;
        text.consume_exact('=')?;

        let value = LogStr::parse_from_sequence(text)?;

        Ok(Self { key, value })
    }

    fn scan_from_with_need_quote(text: &Scanner<'a>, mut f: impl FnMut(char) -> bool) -> Result<Self, ParseError> {
        let key = LogStr::scan_from_with_need_quote(text, &mut f)?;
        text.consume_exact('=')?;

        let value = LogStr::scan_from_with_need_quote(text, &mut f)?;

        Ok(Self { key, value })
    }

    fn parse_from_field<'b : 'a>(text: &'b Scanner<'a>) -> Result<Self, ParseError> {
        text.in_bracket(Self::scan_from)
    }
}

impl LogLevel {
    fn scan_from<'a, 'b:'a>(text: &'b Scanner<'a>) -> Result<Self, ParseError> {
        let field = text.in_bracket(|s| s.till_next_bracket())?;
        Self::from_str(field)
    }
}

impl FromStr for LogLevel {
    type Err = ParseError;

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

// To make tiny vector happy.
impl<'a> Default for LogFieldRef<'a> {
    fn default() -> Self {
        LogFieldRef {
            key: LogStr::Unquoted(""),
            value: LogStr::Unquoted(""),
        }
    }
}

impl<'a> LogRecordRef<'a> {
    fn scan_from<'b: 'a>(scanner: &'b Scanner<'a>) -> Result<Self, ParseError> {
        let time = TimeRef::from_str_unchecked(scanner.in_bracket(|s| s.till_next_bracket())?);
        scanner.skip_space();
        let level= LogLevel::scan_from(&scanner)?;
        scanner.skip_space();
        let source = FileLineRef::scan_from(&scanner)?;
        scanner.skip_space();
        let message = scanner.in_bracket(LogStr::parse_from_sequence)?;
        scanner.skip_space();

        let mut entries = TinyVec::<[LogFieldRef; TINY_VEC_THRESHOLD]>::default();
        while !scanner.is_done() {
            let field = match LogFieldRef::parse_from_field(& scanner) {
                Err(err) => {
                    // TODO use slog!
                    eprintln!("meet error {} during parsing, skipping this field", err);
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

pub fn with_log_record<'a, T: 'a>(s: &'a str, callback: impl FnOnce(LogRecordRef<'_>) -> T) -> Result<T, ParseError> {
    let scanner = Scanner::over(s);
    Ok(callback(LogRecordRef::scan_from(&scanner)?))
}

pub fn with_zap_object<'a, T: 'a>(s: &'a str, callback: impl FnOnce(&[LogFieldRef<'_>]) -> T) -> Result<T, ParseError> {
    let scanner = Scanner::over(s);
    let mut first = true;
    let mut vec = TinyVec::<[_; TINY_VEC_THRESHOLD]>::new();
    scanner.consume_exact('{')?;
    loop {
        if !first {
            scanner.skip_space();
            match scanner.peek_char() {
                Some(',') => scanner.consume_exact(',')?,
                Some('}') => {
                    scanner.consume_exact('}')?;
                    return Ok(callback(&vec))
                }
                Some(any) => {
                    return Err(scanner.unexpected("',' or '}'", any))
                }
                None => return Err(ParseError::Empty),
            }
        }
        first = false;
        let field = LogFieldRef::scan_from_with_need_quote(&scanner, |c| {
            super::scanner::char_need_quote(c) || c == ',' || c == '}' || c == '{'
        })?;
        vec.push(field);
    }
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
    use crate::parser::scanner::Scanner;

    #[test]
    fn test_log_str() {
        use super::LogStr;

        fn check(from: &str, to: &str) {
            let log_str = LogStr::from_str(from);
            assert!(log_str.is_ok());
            let log_str = log_str.unwrap();
            assert_eq!(format!("{}", log_str), to);
        }

    
        check("42??s", r#""42??s""#);
        check(r#""hello, world""#, r#""hello, world""#);
        check("42??s", r#""42??s""#);

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
            let entry = LogFieldRef::scan_from(&scanner);
            assert!(entry.is_ok(), "parsing entry meet error {}", entry.unwrap_err());

            let entry = entry.unwrap();
            assert_eq!(format!("{}", entry.key), key, "failed to check {} (key mismatch {} => {})", from, entry.key, key);
            assert_eq!(format!("{}", entry.value), value, "failed to check {} (value mismatch {} => {})", from, entry.value, value);
        }

        check(r#""start time"=992.547??s""#, r#""start time""#, r#""992.547??s""#);
        check("date-time=12:15:13", r#""date-time""#, r#""12:15:13""#);
        check(r#""rate limit"=128MB/s"#, r#""rate limit""#, r#""128MB/s""#);
        check(r#"emoji????="???????????????????(?????????\"\")???""#, "\"emoji????\"", r#""???????????????????(?????????\"\")???""#);
        let entry = r#""rate l\n\"imit"="128 MB/s""#.to_owned();
        check(&entry, r#""rate l\n\"imit""#, r#""128 MB/s""#);
    }
}
