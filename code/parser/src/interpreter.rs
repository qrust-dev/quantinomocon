use std::{collections::HashMap, cell::RefCell, path::PathBuf, fs};

use pest::Parser;
use qqs::{QuantumSim, sparsestate::SparseState, common_matrices};

use crate::{ast::{Program, FileElement, Statement, Expression, Identifier, Located, Type}, error::{QKaledioscopeError, Result, rule_error_as_parse_error}, parser::{QKaledioscopeParser, Rule}, ast_builder::TryParse};

#[derive(Debug, Clone, Copy)]
pub enum InterpreterValue {
    QubitRef(usize),
    Number(f64),
    Bit(bool),
}

pub type LocalSymbolTable = HashMap<Identifier, InterpreterValue>;

pub enum FunctionTableEntry<'a> {
    Interpreted(&'a Located<FileElement>),
    Builtin(&'a dyn Fn(&[InterpreterValue]) -> Result<Option<InterpreterValue>>),
}

pub struct FunctionTable<'a> {
    // TODO: Use a better type than FileElement here.
    fns: HashMap<Identifier, FunctionTableEntry<'a>>,
}
impl<'a> FunctionTable<'a> {
    pub fn register_builtin<F: 'a + Fn(&[InterpreterValue]) -> Result<Option<InterpreterValue>>>(&mut self, ident: &Identifier, f: &'a F) {
        // TODO: Check if it's already registered, and throw.
        self.fns.insert(ident.clone(), FunctionTableEntry::Builtin(f));
    }

    pub fn build(source: &str, value: &'a Program) -> Result<Self> {
        let mut fns = HashMap::new();
        for element in &value.0 {
            let ident = &match &element.value {
                FileElement::Declaration(prototype) => prototype,
                FileElement::Definition { prototype, body: _ } => prototype
            }.value.name;
            let entry = FunctionTableEntry::Interpreted(element);
            if let Some(existing) = fns.insert(ident.value.clone(), entry) {
                // TODO: Move into util.
                // TODO: Grab only the span for the prototype.
                let (new_start, new_end) = element.location.unwrap();
                let (old_start, old_end) = match existing {
                    FunctionTableEntry::Interpreted(file_element) =>
                        file_element.location.unwrap(),
                    _ => todo!("Spans not yet implemented for built-in.")
                };
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
                FileElement::Definition { prototype: _, body } => {
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
                }
            })
        })
    }

    // TODO: Generalize over simulators with a new trait.
    pub fn run(&self, source: &str) -> Result<()> {
        let sim = RefCell::new(QuantumSim::<SparseState>::new());
        let n_qubits = self.n_qubits_required();
        let n_qubits = 6usize; // FIXME: Don't hard code this.
        println!("Using {n_qubits} qubits...");
        let qubit_ids = (0..n_qubits).map(|_| sim.borrow_mut().allocate()).collect::<Vec<_>>();
        println!("qubit_ids = {qubit_ids:?}");
        let mut table = FunctionTable::build(source, self)?;

        let mk_print = || |args: &[InterpreterValue]| {
            // TODO: Check types and arity here.
            println!("→ {:?}", args[0]);
            Ok(None)
        };
        let print_n = mk_print();
        table.register_builtin(&Identifier("print_n".to_string()), &print_n);
        let print_b = mk_print();
        table.register_builtin(&Identifier("print_b".to_string()), &print_b);
        let print_q = mk_print();
        table.register_builtin(&Identifier("print_q".to_string()), &print_q);

        let h = |args: &[InterpreterValue]| {
            // TODO: Check types and arity here instead of just unpacking...
            match args[0] {
                InterpreterValue::QubitRef(q) => {
                    sim.borrow_mut().apply(&common_matrices::h(), &[q], None);
                },
                _ => panic!("Wrong type for args[0]")
            };
            println!("h({:?})", args[0]);
            Ok(None)
        };
        table.register_builtin(&Identifier("h".to_string()), &h);

        let cnot = |args: &[InterpreterValue]| {
            // TODO: Check types and arity here instead of just unpacking...
            let c = match args[0] {
                InterpreterValue::QubitRef(q) => q,
                _ => panic!("Wrong type for args[0]")
            };
            let t = match args[1] {
                InterpreterValue::QubitRef(q) => q,
                _ => panic!("Wrong type for args[0]")
            };
            sim.borrow_mut().apply(&common_matrices::x(), &[t], Some(&[c]));
            println!("cnot({:?})", args[0]);
            Ok(None)
        };
        table.register_builtin(&Identifier("cnot".to_string()), &cnot);

        let m = |args: &[InterpreterValue]| {
            // TODO: Check types and arity here instead of just unpacking...
            let r = match args[0] {
                InterpreterValue::QubitRef(q) => {
                    sim.borrow_mut().measure(q)
                },
                _ => panic!("Wrong type for args[0]")
            };
            println!("m({:?}) -> {r}", args[0]);
            Ok(Some(InterpreterValue::Bit(r)))
        };
        table.register_builtin(&Identifier("m".to_string()), &m);

        let qmain = table
            .fns
            .get(&Identifier("qmain".to_string()))
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
                let function = fn_table.fns.get(&ident.value).unwrap();
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

impl FunctionTableEntry<'_> {
    // TODO: Add args here.
    pub fn run_in(&self, source: &str, table: &FunctionTable, args: Vec<InterpreterValue>) -> Result<Option<InterpreterValue>> {
        match self {
            FunctionTableEntry::Builtin(f) =>
                f(&args),
            FunctionTableEntry::Interpreted(file_element) => match &file_element.value {
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
                            },
                            Statement::Call(ident, args) => {
                                // TODO: Deduplicate with Expression::Call case.      
                                // TODO: raise nice error instead of unwrapping.
                                let function = table.fns.get(&ident.value).unwrap();
                                // We don't use map here so that we can more easily break out on first error...
                                // it doesn't make sense to continue interpreting past a crash.
                                let mut arg_values = vec![];
                                for arg in args.iter() {
                                    arg_values.push(arg.eval_in(source, table, &mut symbol_table)?);
                                }
                                // TODO: Check if the return is some, raise an error.
                                function.run_in(source, table, arg_values)?;
                            },
                            _ => todo!()
                        }
                    }
                    Ok(None)
                }
            }
        }
    }
}

pub fn run(source_file: PathBuf) -> miette::Result<()> {
    // TODO: Extract common functionality.
    let fname = source_file.to_str().map(|s| s.to_string());
    let source = fs::read_to_string(&source_file).map_err(|e| QKaledioscopeError::IOError {
        cause: e,
        subject: fname
    })?;
    let source = source.as_str();
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

    let program = Program(program);
    program.run(&source)?;

    Ok(())
}
