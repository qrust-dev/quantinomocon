use crate::ast::{
    ArgumentDeclaration, Expression, FileElement, Identifier, Located, Program, Prototype,
    Statement, Type,
};
use crate::error::{
    rule_error_as_parse_error, wrong_rule_as_parse_error, QKaledioscopeError, Result,
};
use crate::parser::{QKaledioscopeParser, Rule};
use crate::util::ResultIter;
use pest::iterators::Pair;
use pest::{Parser, Span};
use std::vec;
use std::{fmt::Debug, str::FromStr};

pub(crate) trait TryParse
where
    Self: Sized + Debug,
{
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Self>;
    fn try_parse(source: &str, pair: Pair<Rule>) -> Result<Located<Self>> {
        let span = pair.as_span();
        let raw = Self::try_parse_raw(source, pair)?;
        Ok(Located {
            value: raw,
            location: Some(
                (span.start(), span.end())
            ),
        })
    }

    fn try_parse_many<'a, I: Iterator<Item = Pair<'a, Rule>>>(
        source: &str,
        span: Span,
        pairs: &mut I,
    ) -> Result<Vec<Located<Self>>> {
        match pairs
            .map(|pair| Self::try_parse(source, pair))
            .try_collect()
        {
            Ok(many) => Ok(many),
            Err(errs) => Err(wrong_rule_as_parse_error(
                source,
                "Expected definition body",
                span,
                errs,
            )),
        }
    }
}

impl TryParse for FileElement {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<FileElement> {
        match pair.as_rule() {
            Rule::declaration => {
                Prototype::try_parse(source, pair.into_inner().next().unwrap())
                    .map(|ok| FileElement::Declaration(ok))
            }
            Rule::definition => {
                let span = pair.as_span();
                let mut inner = pair.into_inner();
                let proto = Prototype::try_parse(source, inner.next().unwrap())?;
                let body = Statement::try_parse_many(source, span, &mut inner)?;
                Ok(FileElement::Definition {
                    prototype: proto,
                    body,
                })
            }
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected declaration or definition.",
                pair.as_span(),
                vec![],
            )),
        }
    }
}

impl TryParse for Prototype {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Prototype> {
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
                        Some(Type::try_parse(
                            source,
                            pair.into_inner().next().unwrap(),
                        )?)
                    }
                    None => None,
                };
                Ok(Prototype {
                    name: ident,
                    arguments,
                    return_type,
                })
            }
        }
    }
}


impl TryParse for ArgumentDeclaration {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::arg_decl => {
                let mut pairs = pair.into_inner();
                let ident = Identifier::try_parse(source, pairs.next().unwrap())?;
                let type_sig = Type::try_parse(source, pairs.next().unwrap())?;
                Ok(ArgumentDeclaration(ident, type_sig))
            }
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected argument declaration",
                pair.as_span(),
                vec![],
            )),
        }
    }
}

impl TryParse for Type {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Self> {
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
        }
    }
}

impl TryParse for Identifier {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Identifier> {
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

impl TryParse for Statement {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::variable_declaration => {
                let mut inner = pair.into_inner();
                let ident = Identifier::try_parse(source, inner.next().unwrap())?;
                let type_sig = Type::try_parse(source, inner.next().unwrap())?;
                let value = Expression::try_parse(source, inner.next().unwrap())?;
                Ok(Statement::VariableDeclaration(ident, type_sig, value))
            },
            Rule::assignment => {
                let mut inner = pair.into_inner();
                let ident = Identifier::try_parse(source, inner.next().unwrap())?;
                let value = Expression::try_parse(source, inner.next().unwrap())?;
                Ok(Statement::Assignment(ident, value))
            },
            Rule::call_expr => {
                let span = pair.as_span();
                let mut inner = pair.into_inner();
                let ident = Identifier::try_parse(source, inner.next().unwrap())?;
                let arguments = Expression::try_parse_many(source, span, &mut inner)?;
                Ok(Statement::Call(ident, arguments))
            },
            Rule::if_stmt => {
                let mut inner = pair.into_inner();

                let if_block = inner.next().unwrap();
                let if_span = if_block.as_span();
                let mut if_block = if_block.into_inner();
                let condition = Expression::try_parse(source, if_block.next().unwrap())?;
                let true_body = Statement::try_parse_many(source, if_span, &mut if_block)?;

                let else_block = inner.next(); // NB: don't unwrap here, since else_block is optional.
                let else_body = if let Some(else_block) = else_block {
                    let else_span = else_block.as_span();
                    Statement::try_parse_many(source, else_span, &mut else_block.into_inner())?
                } else {
                    vec![]
                };


                Ok(Statement::If {
                    condition,
                    true_body: true_body,
                    false_body: else_body,
                })
            },
            Rule::while_stmt => {
                let span = pair.as_span();
                let mut inner = pair.into_inner();
                let condition = Expression::try_parse(source, inner.next().unwrap())?;
                let body = Statement::try_parse_many(source, span, &mut inner)?;
                Ok(Statement::While {
                    condition, body
                })
            },
            Rule::return_stmt => {
                let mut inner = pair.into_inner();
                let value = Expression::try_parse(source, inner.next().unwrap())?;
                Ok(Statement::Return(value))
            }
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected a valid statement",
                pair.as_span(),
                vec![],
            )),
        }
    }
}

impl TryParse for Expression {
    fn try_parse_raw(source: &str, pair: Pair<Rule>) -> Result<Self> {
        match pair.as_rule() {
            Rule::call_expr => {
                let span = pair.as_span();
                let mut inner = pair.into_inner();
                let ident = Identifier::try_parse(source, inner.next().unwrap())?;
                let arguments = Expression::try_parse_many(source, span, &mut inner)?;
                Ok(Expression::Call(ident, arguments))
            },
            Rule::Ident => {
                Ok(Expression::Identifier(Identifier::try_parse_raw(source, pair)?))
            },
            Rule::TrueKeyword => Ok(Expression::BitLiteral(true)),
            Rule::FalseKeyword => Ok(Expression::BitLiteral(false)),
            Rule::number_literal => Ok({
                let span = pair.as_span();
                let s = pair.as_str();
                let val = f64::from_str(s).map_err(|e| {
                    wrong_rule_as_parse_error(
                        source,
                        format!("Could not convert `{}` to number literal", s).as_str(),
                        span,
                        vec![QKaledioscopeError::ParseFloatError(e)],
                    )
                })?;
                Expression::NumberLiteral(val)
            }),
            Rule::qubit_literal => Ok({
                let span = pair.as_span();
                let s = pair.as_str();
                let idx = usize::from_str(&s[1..]).map_err(|e| {
                    wrong_rule_as_parse_error(
                        source,
                        format!("Could not convert `{}` to qubit literal", s).as_str(),
                        span,
                        vec![QKaledioscopeError::ParseIntError(e)],
                    )
                })?;
                Expression::QubitLiteral(idx)
            }),
            _ => Err(wrong_rule_as_parse_error(
                source,
                "Expected a valid expression",
                pair.as_span(),
                vec![],
            )),
        }
    }
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
