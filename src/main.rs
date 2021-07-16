#![feature(never_type)]

use std::{error, io::{BufRead, Error as IoError, Write}};
use tidc::{json_writer::ToJSON, parser::artifacts::{with_log_record}};

type Error = Box<dyn error::Error>;


fn run_from_stdin() -> Result<(), Error> {
    let stdin = std::io::stdin();
    let inputs = stdin.lock();
    let stdout = std::io::stdout();
    let mut outputs = stdout.lock();
    
    for line in inputs.lines() {
        let line = line?;
        with_log_record(&line, |r| -> Result<(), Error> {
            r.write_json_to(&mut outputs)?;
            writeln!(outputs)?;
            Ok(())
        })??;
    }
    Ok(())
}

fn main() -> Result<(), Error>{
    match run_from_stdin() {
        Err(e) => {
            match e.downcast::<IoError>() {
                Ok(os_err) => match os_err.kind() {
                    std::io::ErrorKind::BrokenPipe => {
                        // don't report error for BrokenPipe.
                        return Ok(())
                    }
                    _ => return Err(os_err)
                }
                Err(e) => return Err(e)
            }
        }
        Ok(()) => {
            return Ok(());
        }
    }
}
