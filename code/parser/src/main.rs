#[macro_use]
extern crate pest_derive;

use std::{path::PathBuf};
use clap::{self, StructOpt};

// NB: The modules below are listed roughly in the order that's easiest to
//     read and follow along with. Each module requires mostly only what's
//     defined in previously modules; for example, `ast` depends on `parser`,
//     but not the other way around. 

pub mod parser;
pub mod ast;
pub mod ast_builder;
pub mod interpreter;
pub mod error;
pub mod codegen;

mod util;

#[derive(clap::Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand, Debug)]
pub enum Action {
    /// Parses a Quantum Kalediscope program and prints the result.
    Parse {
        source_file: PathBuf,
    },
    /// Parses a Quantum Kalediscope program and prints an abstract syntax tree
    /// for the program.
    BuildAst {
        source_file: PathBuf,
    },
    /// Interprets a Quantum Kalediscope program and runs it on a full-state
    /// quantum simulator.
    Interpret {
        source_file: PathBuf,
    },
    Compile {
        source_file: PathBuf,
        // TODO: output file
        // TODO: verbosity
    }
}

fn main() -> miette::Result<()> {
    let args = Args::parse();
    match args.action {
        Action::Parse { source_file } => parser::run_parse_cmd(source_file),
        Action::BuildAst { source_file } => ast_builder::run_build_cmd(source_file),
        Action::Interpret { source_file } => interpreter::run_interpret_cmd(source_file),
        Action::Compile { source_file } => codegen::run_compile_cmd(source_file),
    }


    // let fname = args.source_file.to_str().map(|s| s.to_string());
    // let source = fs::read_to_string(args.source_file).map_err(|e| QKaledioscopeError::IOError {
    //     cause: e,
    //     subject: fname
    // })?;
    // let ast = parse(&source)?;

    // println!("Parsed into AST: {:?}", ast);

    // let table = FunctionTable::build(&source, &ast)?;
    // println!("Built function table: {:?}", table);

    // ast.run(&source)?;

    // Ok(())
}
