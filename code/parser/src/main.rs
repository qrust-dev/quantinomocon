#[macro_use]
extern crate pest_derive;

use std::{fs, path::PathBuf};
use clap::{self, StructOpt};

// NB: The modules below are listed roughly in the order that's easiest to
//     read and follow along with. Each module requires mostly only what's
//     defined in previously modules; for example, `ast` depends on `parser`,
//     but not the other way around. 

pub mod parser;
use parser::*;

pub mod ast;
use ast::*;

pub mod ast_builder;
use ast_builder::*;

pub mod interpreter;
use interpreter::*;

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

    // println!("Parsed into AST: {:?}", ast);

    // let table = FunctionTable::build(&source, &ast)?;
    // println!("Built function table: {:?}", table);

    ast.run(&source)?;

    Ok(())
}
