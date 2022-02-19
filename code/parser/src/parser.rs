use std::{path::PathBuf, fs};

use pest::{Parser, iterators::Pairs};

use crate::error::{QKaledioscopeError, rule_error_as_parse_error, Result};

#[derive(Parser, Debug)]
#[grammar = "qkaledioscope.pest"]
pub struct QKaledioscopeParser;

pub fn parse<'a>(source: &'a str) -> Result<Pairs<'a, Rule>> {
    let pairs = QKaledioscopeParser::parse(Rule::program, source)
        .map_err(|e| rule_error_as_parse_error(source, e))?;
    Ok(pairs)
}

pub fn run_parse_cmd(source_file: PathBuf)  -> miette::Result<()> {
    let fname = source_file.to_str().map(|s| s.to_string());
    let source = fs::read_to_string(&source_file).map_err(|e| QKaledioscopeError::IOError {
        cause: e,
        subject: fname
    })?;

    let pairs = parse(source.as_str())?;
    let json = pairs.to_json();

    println!("{json}");
    Ok(())
}
