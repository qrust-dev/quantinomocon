#[macro_use]
extern crate pest_derive;

use std::{fs, path::PathBuf};

use clap::{self, StructOpt};

pub mod ast;
use ast::*;

pub mod error;
use crate::error::QKaledioscopeError;

mod util;


#[derive(clap::Parser, Debug)]
struct Args {
    source_file: PathBuf,
}

fn main() -> miette::Result<()> {
    let args = Args::parse();
    let fname = args.source_file.to_str().map(|s| s.to_string());
    let source = fs::read_to_string(args.source_file).map_err(|e| QKaledioscopeError::IOError {
        cause: e,
        subject: fname
    })?;
    let ast = parse(&source)?;

    println!("Parsed into AST: {:?}", ast);
    Ok(())
}
