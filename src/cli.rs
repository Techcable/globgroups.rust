use std::error::Error;
use std::ffi::OsString;

use globgroups::{GlobExpr, GlobParseError};

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("Too many arguments: {0}, only need one")]
    TooManyArguments(usize),
    #[error("Insufficient arguments: {0}. Need to specify glob pattern")]
    InsufficientArguments(usize),
    #[error("Invalid glob pattern")]
    ParseError(#[from] GlobParseError),
}

pub fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("ERROR: {e}");
            let mut source_opt = e.source();
            while let Some(source) = source_opt {
                eprintln!("Cause: {source}");
                source_opt = source.source();
            }

            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), CliError> {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let glob_spec = match args.len() {
        0 => return Err(CliError::InsufficientArguments(0)),
        1 => &args[0],
        many => return Err(CliError::TooManyArguments(many)),
    };
    let glob: GlobExpr = glob_spec.parse()?;
    if std::env::var_os("DEBUG_PARSE_GLOB") == Some(OsString::from("1")) {
        eprintln!("parsed_glob: {glob:#?}");
    }
    for text in glob.expand() {
        println!("{text}");
    }
    Ok(())
}
