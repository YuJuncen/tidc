pub mod parser;
pub mod json_writer;

use std::io;
use crate::parser::ParseError;
use quick_error::quick_error;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            from()
            source(err)
            display("I/O Error: {}", err) 
        }
        Parse(err: ParseError) {
            from()
            source(err)
            display("Error during parsing log: {}", err)
        }
        Cli(msg: String) {
            display("CLI interface error: {}", msg)
        }
    }
}