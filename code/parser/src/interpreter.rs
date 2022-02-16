use std::{collections::HashMap, hash::Hash};

use qqs::{QuantumSim, sparsestate::SparseState};

use crate::{ast::{Program, FileElement, Statement, Expression, Identifier, Located, Type}, error::{QKaledioscopeError, Result}, util::ResultIter};

#[derive(Debug, Clone, Copy)]
pub enum InterpreterValue {
    QubitRef(usize),
    Number(f64),
    Bit(bool),
}

pub type LocalSymbolTable = HashMap<Identifier, InterpreterValue>;
pub enum FunctionTableEntry<'a> {
    Interpreted(&'a Located<FileElement>),
    Builtin(&'a dyn Fn(&mut LocalSymbolTable)),
}

#[derive(Debug)]
pub struct FunctionTable<'a> {
    // TODO: Use a better type than FileElement here.
    fns: HashMap<Identifier, &'a Located<FileElement>>,
}
impl<'a> FunctionTable<'a> {
    pub fn build(source: &str, value: &'a Program) -> Result<Self> {
        let mut fns = HashMap::new();
        for element in &value.0 {
            let ident = &match &element.value {
                FileElement::Declaration(prototype) => prototype,
                FileElement::Definition { prototype, body: _ } => prototype
            }.value.name;
            if let Some(existing) = fns.insert(ident.value.clone(), element) {
                // TODO: Move into util.
                // TODO: Grab only the span for the prototype.
                let (new_start, new_end) = element.location.unwrap();
                let (old_start, old_end) = existing.location.unwrap();
                return Err(QKaledioscopeError::DuplicateNameError {
                    src: source.to_string(),
                    name: ident.value.0.clone(),
                    // FIXME: Don't unwrap here!
                    new_span: (new_start, new_end - new_start),
                    old_span: (old_start, old_end - old_start),
                })
            }
        }
        Ok(FunctionTable { fns })
    }
}

// TODO: n_qubits_required currently takes the max over all qubit literals, but
//       could use a map onto IDs instead.
impl Expression {
    fn n_qubits_required(&self) -> usize {
        match self {
            Expression::BitLiteral(_) => 0,
            Expression::NumberLiteral(_) => 0,
            Expression::Identifier(_) => 0,
            Expression::QubitLiteral(idx) => *idx,
            Expression::Call(_, arguments) =>
                arguments.iter().fold(0, |acc, expr| {
                    std::cmp::max(acc, expr.value.n_qubits_required())
                })
        }
    }
}

impl Program {
    fn n_qubits_required(&self) -> usize {
        self.0.iter().fold(0, |acc, element| {
            std::cmp::max(acc, match &element.value {
                FileElement::Declaration(_) => 0,
                FileElement::Definition { prototype, body } => {
                    body.iter().fold(0, |acc, stmt| {
                        std::cmp::max(acc, match &stmt.value {
                            Statement::Assignment(_, expr) => expr.value.n_qubits_required(),
                            Statement::Call(_, arguments) => 
                                arguments.iter().fold(0, |acc, expr| {
                                    std::cmp::max(acc, expr.value.n_qubits_required())
                                }),
                            // TODO: Scan in other statement types as well!
                            _ => 0
                        })
                    })
                },
                _ => 0
            })
        })
    }

    // TODO: Generalize over simulators with a new trait.
    pub fn run(&self, source: &str) -> Result<()> {
        let mut sim = QuantumSim::<SparseState>::new();
        let n_qubits = self.n_qubits_required();
        let n_qubits = 6usize; // FIXME: Don't hard code this.
        println!("Using {n_qubits} qubits...");
        let qubit_ids = (0..n_qubits).map(|_| sim.allocate());
        let table = FunctionTable::build(source, self)?;

        let qmain = *(table
            .fns
            .get(&Identifier("qmain".to_string())))
            .ok_or(QKaledioscopeError::NoQMainError)?;

        qmain.run_in(source, &table, vec![])?;

        Ok(())
    }
}

impl Located<Expression> {
    pub fn eval_in(&self, source: &str, fn_table: &FunctionTable, symbol_table: &mut LocalSymbolTable) -> Result<InterpreterValue> {
        Ok(match &self.value {
            Expression::BitLiteral(bit) => InterpreterValue::Bit(*bit),
            Expression::NumberLiteral(num) => InterpreterValue::Number(*num),
            Expression::QubitLiteral(idx) => InterpreterValue::QubitRef(*idx),
            Expression::Identifier(ident) => {
                let value = *(symbol_table.get(&ident).ok_or(QKaledioscopeError::UndefinedVariableError {
                    name: ident.0.clone(),
                    src: source.to_string(),
                    span: self.as_sourcespan(),
                })?);
                value
            },
            Expression::Call(ident, args) => {
                // TODO: raise nice error instead of unwrapping.
                let function = *(fn_table.fns.get(&ident.value).unwrap());
                // We don't use map here so that we can more easily break out on first error...
                // it doesn't make sense to continue interpreting past a crash.
                let mut arg_values = vec![];
                for arg in args.iter() {
                    arg_values.push(arg.eval_in(source, fn_table, symbol_table)?);
                }
                // TODO: Check if the return is none and raise a nice error.
                function.run_in(source, fn_table, arg_values)?.unwrap()
            }
        })
    }
}

impl Located<FileElement> {
    // TODO: Add args here.
    pub fn run_in(&self, source: &str, table: &FunctionTable, args: Vec<InterpreterValue>) -> Result<Option<InterpreterValue>> {
        match &self.value {
            // TODO: Try looking up extern.
            FileElement::Declaration(prototype) => Err(QKaledioscopeError::LinkingError {
                name: prototype.value.name.value.0.to_string(),
                src: source.to_string(),
                // TODO: Don't unwrap here.
                span: (prototype.location.unwrap().0, prototype.location.unwrap().1 - prototype.location.unwrap().0)
            }),
            // TODO: populate args into symbol table, using prototype.
            FileElement::Definition { prototype, body } => {
                let mut symbol_table = LocalSymbolTable::new();
                // TODO: Validate prototypes don't have repeated identifiers.
                // TODO: Validate right number and types of args.
                for (ident, arg) in prototype.value.arguments.iter().zip(args) {
                    symbol_table.insert(ident.value.0.value.clone(), arg);
                }
                for statement in body {
                    match &statement.value {
                        Statement::VariableDeclaration(ident, type_sig, expr) => {
                            let value = expr.eval_in(source, table, &mut symbol_table)?;
                            match (&type_sig.value, &value) {
                                (Type::Bit, InterpreterValue::Bit(_)) => Ok(()),
                                (Type::Number, InterpreterValue::Number(_)) => Ok(()),
                                (Type::Qubit, InterpreterValue::QubitRef(_)) => Ok(()),
                                _ => Err(QKaledioscopeError::TypeError {
                                    // TODO: Nicer printouts for these types.
                                    expected: format!("{:?}", &type_sig.value).to_string(),
                                    actual: match &value {
                                        InterpreterValue::Bit(_) => "bit",
                                        InterpreterValue::Number(_) => "number",
                                        InterpreterValue::QubitRef(_) => "qubit"
                                    }.to_string(),
                                    expr_span: expr.as_sourcespan(),
                                    type_span: type_sig.as_sourcespan(),
                                    src: source.to_string()
                                })
                            }?;
                            // TODO: Check if the variable was already defined and throw if so.
                            symbol_table.insert(ident.value.clone(), value);
                            println!("symbol_table: {symbol_table:?}");
                        },
                        Statement::Return(expr) => {
                            let value = expr.eval_in(source, table, &mut symbol_table)?;
                            return Ok(Some(value));
                        }
                        _ => todo!()
                    }
                }
                Ok(None)
            }
        }
    }
}