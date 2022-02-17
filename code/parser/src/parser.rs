use std::{path::PathBuf, fs};

use pest::Parser;

use crate::error::{QKaledioscopeError, rule_error_as_parse_error};

#[derive(Parser, Debug)]
#[grammar = "qkaledioscope.pest"]
pub struct QKaledioscopeParser;

pub fn parse(source_file: PathBuf)  -> miette::Result<()> {
    let fname = source_file.to_str().map(|s| s.to_string());
    let source = fs::read_to_string(&source_file).map_err(|e| QKaledioscopeError::IOError {
        cause: e,
        subject: fname
    })?;
    let pairs = QKaledioscopeParser::parse(Rule::program, &source)
        .map_err(|e| rule_error_as_parse_error(source.as_str(), e))?;
    let json = pairs.to_json();

    println!("{json}");
    Ok(())
}
