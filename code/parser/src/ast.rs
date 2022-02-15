use miette::SourceSpan;
use pest::{Parser, iterators::Pair};
use crate::{error::{Result, wrong_rule_as_parse_error, rule_error_as_parse_error}, util::ResultIter};

#[derive(Parser, Debug)]
#[grammar = "qkaledioscope.pest"]
pub struct QKaledioscopeParser;

pub(crate) trait TryParse where Self: Sized {
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<Self>;
}


#[derive(Debug)]
pub struct Program(Vec<FileElement>);

#[derive(Debug)]
pub enum FileElement {
    Declaration(Prototype),
    // TODO: Finish adding items to Definition.
    Definition {
        prototype: Prototype,
        body: Vec<Statement>
    },
}
impl TryParse for FileElement {
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<FileElement> {
        match pair.as_rule() {
            Rule::declaration => {
                Prototype::try_parse(
                    source, pair.into_inner().next().unwrap(),
                ).map(|ok| FileElement::Declaration(ok))
            },
            Rule::definition => {
                Prototype::try_parse(
                    source, pair.into_inner().next().unwrap()
                )
                .map(|ok| FileElement::Definition {
                    prototype: ok,
                    body: vec![]
                })
            },
            _ => {
                Err(wrong_rule_as_parse_error(
                    source,
                    "Expected declaration or definition.",
                    pair.as_span(),
                    vec![],
                ))
            },
        }
    }
}

#[derive(Debug)]
pub struct Prototype {
    pub name: Identifier,
    pub arguments: Vec<ArgumentDeclaration>,
    pub return_type: Option<Type>,
}
impl TryParse for Prototype {
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<Prototype> {
        let span = pair.as_span();
        if !matches!(pair.as_rule(), Rule::prototype) {
            return Err(wrong_rule_as_parse_error(
                source,
                "Expected prototype.",
                pair.as_span(),
                vec![],
            ));
        }
        let mut pairs = pair.into_inner();
        let ident = Identifier::try_parse(source, pairs.next().unwrap())?;
        let arg_pairs = pairs.next().unwrap().into_inner();
        match arg_pairs
            .map(|pair| ArgumentDeclaration::try_parse(source, pair))
            .try_collect()
        {
            Err(errs) => Err(wrong_rule_as_parse_error(
                source,
                "Expected argument declarations",
                span,
                errs,
            )),
            Ok(arguments) => {
                let return_type = match pairs.next() {
                    Some(pair) => {
                        // Unpack the return_decl as well.
                        Some(Type::try_parse(source, pair.into_inner().next().unwrap())?)
                    },
                    None => None
                };
                Ok(Prototype {
                    name: ident,
                    arguments,
                    return_type
                })
            },
        }
    }
}

#[derive(Debug)]
pub struct ArgumentDeclaration(Identifier, Type);
impl TryParse for ArgumentDeclaration {
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::arg_decl => {
                let mut pairs = pair.into_inner();
                let ident = Identifier::try_parse(source, pairs.next().unwrap())?;
                let type_sig = Type::try_parse(source, pairs.next().unwrap())?;
                Ok(ArgumentDeclaration(ident, type_sig))
            },
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected argument declaration",
                pair.as_span(),
                vec![],
            )),
        }
        
    }
}

#[derive(Debug)]
pub enum Type {
    Number,
    Qubit,
    Bit
}
impl TryParse for Type {
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::qubit_type => Ok(Type::Qubit),
            Rule::number_type => Ok(Type::Number),
            Rule::bit_type => Ok(Type::Bit),
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected a valid type",
                pair.as_span(),
                vec![],
            )),
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected type",
                pair.as_span(),
                vec![],
            )),
        }
    }
}

#[derive(Debug)]
pub struct Identifier(String);
impl TryParse for Identifier {
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<Identifier> {
        match pair.as_rule() {
            Rule::Ident => Ok(Identifier(pair.as_str().to_string())),
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected identifier",
                pair.as_span(),
                vec![],
            )),
        }
    }
}

#[derive(Debug)]
pub struct Statement(Expression);

#[derive(Debug)]
pub enum Expression{
    /// Represents parenthesized subexpressions.
    Group,
    // TODO: Add arguments.
    Call(Identifier,),
    Identifier(Identifier),
    Literal // TODO: Add what literal it is!
}

pub fn parse(source: &str) -> Result<Program> {
    let mut program = vec![];

    let pairs = QKaledioscopeParser::parse(Rule::program, source)
        .map_err(|e| rule_error_as_parse_error(source, e))?;
    for pair in pairs {
        // Ignore the end of the file, but try to parse everything else.
        if !matches!(pair.as_rule(), Rule::EOI) {
            // TODO: write util fn to try parse multiple.
            let element = FileElement::try_parse(source, pair)?;
            program.push(element);
        }
    }
    Ok(Program(program))
}
