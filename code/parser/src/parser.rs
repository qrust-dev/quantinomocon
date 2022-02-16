use pest::{Parser};

#[derive(Parser, Debug)]
#[grammar = "qkaledioscope.pest"]
pub struct QKaledioscopeParser;
