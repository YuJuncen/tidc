#![feature(never_type)]

use std::{io::{self, BufRead, Error as IoError, Write}};
use tidc::{json_writer::ToJSON, parser::{artifacts::{with_log_record, with_zap_object}}};
use structopt::StructOpt;

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

fn zap_object_from_stdin() -> Result<(), tidc::Error> {
    let stdin = std::io::stdin();
    let inputs = stdin.lock();
    let stdout = std::io::stdout();
    let mut outputs = stdout.lock();

    for line in inputs.lines() {
        let line = line?;
        with_zap_object(&line, |r| -> Result<(), IoError> {
            r.write_json_to(&mut outputs)?;
            writeln!(outputs)?;
            Ok(())
        })??;
    }
    Ok(())
}

/// on_cli_error handles the error during the cli running.
fn on_cli_error(e: tidc::Error) -> Result<(), tidc::Error> {
    match e {
        tidc::Error::Io(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        e => Err(e),
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "tidc", about = "A minimal decoder for TiKV uniformed log format.")]
struct Opt {
    #[structopt(default_value = "uniformed-log")]
    decoder: String
}

fn main() -> Result<(), tidc::Error>{
    let opt = Opt::from_args();
    let result = match opt.decoder.as_str() {
        "uniformed-log" => run_from_stdin(),
        "zap-object" => zap_object_from_stdin(),
        other => return Err(tidc::Error::Cli(format!("decoder {} isn't supported", other)))
    };
    match result {
        Err(e) => on_cli_error(e),
        Ok(()) => Ok(())
    }
}
