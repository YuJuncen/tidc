#![feature(never_type)]

use std::{io::{self, BufRead, Error as IoError, Write}};
use tidc::{json_writer::ToJSON, parser::{artifacts::{with_log_record}}};


fn run_from_stdin() -> Result<(), tidc::Error> {
    let stdin = std::io::stdin();
    let inputs = stdin.lock();
    let stdout = std::io::stdout();
    let mut outputs = stdout.lock();
    
    for line in inputs.lines() {
        let line = line?;
        with_log_record(&line, |r| -> Result<(), IoError> {
            r.write_json_to(&mut outputs)?;
            writeln!(outputs)?;
            Ok(())
        })??;
    }
    Ok(())
}

fn main() -> Result<(), tidc::Error>{
    match run_from_stdin() {
        Err(tidc::Error::Io(e)) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Ok(())
    }
}
