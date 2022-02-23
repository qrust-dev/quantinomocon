use std::{collections::HashMap, path::PathBuf, hash::Hash, fs::File};

use either::Either;
use inkwell::{context::Context, builder::Builder, passes::PassManager, values::{FunctionValue, PointerValue, BasicValue, IntValue, FloatValue, StructValue, BasicMetadataValueEnum, BasicValueEnum, InstructionOpcode, InstructionValue}, module::Module, types::{StructType, BasicTypeEnum, FunctionType, FloatType, VoidType, IntType, BasicMetadataTypeEnum, BasicType, PointerType}, basic_block::BasicBlock};
use miette::IntoDiagnostic;

use crate::{ast::{FileElement, Program, Prototype, Located, Type, ArgumentDeclaration, Statement, Expression, Identifier}, error::Result, ast_builder::build_ast};

// NB: We largely follow the inkwell::kaledioscope tutorial at
//     https://github.com/TheDan64/inkwell/blob/master/examples/kaleidoscope/main.rs
//     in developing this module.

pub trait ReturnType<'ctx> {
    fn func_type(&self, param_types: &[BasicMetadataTypeEnum<'ctx>], is_var_args: bool) -> FunctionType<'ctx>;
}
impl<'ctx> ReturnType<'ctx> for PointerType<'ctx> {
    fn func_type(&self, param_types: &[BasicMetadataTypeEnum<'ctx>], is_var_args: bool) -> FunctionType<'ctx> {
        self.fn_type(param_types, is_var_args)
    }
}
impl<'ctx> ReturnType<'ctx> for FloatType<'ctx> {
    fn func_type(&self, param_types: &[BasicMetadataTypeEnum<'ctx>], is_var_args: bool) -> FunctionType<'ctx> {
        self.fn_type(param_types, is_var_args)
    }
}
impl<'ctx> ReturnType<'ctx> for VoidType<'ctx> {
    fn func_type(&self, param_types: &[BasicMetadataTypeEnum<'ctx>], is_var_args: bool) -> FunctionType<'ctx> {
        self.fn_type(param_types, is_var_args)
    }
}
impl<'ctx> ReturnType<'ctx> for IntType<'ctx> {
    fn func_type(&self, param_types: &[BasicMetadataTypeEnum<'ctx>], is_var_args: bool) -> FunctionType<'ctx> {
        self.fn_type(param_types, is_var_args)
    }
}
impl<'ctx> ReturnType<'ctx> for StructType<'ctx> {
    fn func_type(&self, param_types: &[BasicMetadataTypeEnum<'ctx>], is_var_args: bool) -> FunctionType<'ctx> {
        self.fn_type(param_types, is_var_args)
    }
}

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub fpm: &'a PassManager<FunctionValue<'ctx>>,
    pub module: &'a Module<'ctx>,
    pub program: &'a Program,

    variables: HashMap<String, PointerValue<'ctx>>,
    fn_value_opt: Option<FunctionValue<'ctx>>
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Gets a defined function given its name.
    #[inline]
    fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.module.get_function(name)
    }

    /// Returns the `FunctionValue` representing the function being compiled.
    #[inline]
    fn fn_value(&self) -> FunctionValue<'ctx> {
        self.fn_value_opt.unwrap()
    }

    fn get_or_define_struct(
        &self,
        name: &str,
    ) -> StructType<'ctx> {
        if let Some(struct_type) = self.module.get_struct_type(name) {
            struct_type
        } else {
            self.context.opaque_struct_type(name)
        }
    }

    fn qubit_type(&self) -> PointerType<'ctx> {
        self.get_or_define_struct("Qubit").ptr_type(inkwell::AddressSpace::Generic)
    }

    /// Creates a new stack allocation instruction in the entry block of the function.
    fn create_entry_block_alloca(&self, name: &str, ty: &Type) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();

        let entry = self.fn_value().get_first_basic_block().unwrap();

        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry)
        }

        match ty {
            Type::Bit => builder.build_alloca(self.context.bool_type(), name),
            Type::Number => builder.build_alloca(self.context.f64_type(), name),
            Type::Qubit => builder.build_alloca(self.qubit_type(), name),
        }
    }

    // TODO: Change Result to crate::error::Result by adding appropriate
    //       cases to QKaledioscopeError.
    fn compile_prototype(&self, proto: &Prototype) -> std::result::Result<FunctionValue<'ctx>, &'static str> {
        let ret_type: Box<dyn ReturnType> = match &proto.return_type {
            None => Box::new(self.context.void_type()),
            Some(Located { value, location }) => match value {
                Type::Bit => Box::new(self.context.bool_type()),
                Type::Number => Box::new(self.context.f64_type()),
                Type::Qubit => Box::new(self.qubit_type()),
            }
        };

        let (arg_names, arg_types): (Vec<_>, Vec<BasicMetadataTypeEnum>) = proto
            .arguments
            .iter()
            .map(|arg| {
                let ArgumentDeclaration(ident, type_sig) = &arg.value;
                (
                    ident.value.0.clone(),
                    match type_sig.value {
                        Type::Bit => BasicMetadataTypeEnum::IntType(self.context.bool_type()),
                        Type::Number => BasicMetadataTypeEnum::FloatType(self.context.f64_type()),
                        Type::Qubit => BasicMetadataTypeEnum::PointerType(self.qubit_type()),
                    }
                )
            })
            .unzip();

        let fn_type = ret_type.func_type(arg_types.as_slice(), false);
        let fn_val = self.module.add_function(proto.name.value.0.as_str(), fn_type, None);

        for (arg, arg_name) in fn_val.get_param_iter().zip(arg_names) {
            arg.set_name(&arg_name.as_str())
        }

        Ok(fn_val)
    }

    fn compile_call(&mut self, ident: &Located<Identifier>, arg_exprs: &[Located<Expression>]) -> Either<BasicValueEnum<'ctx>, InstructionValue<'ctx>> {
        // TODO: Don't unwrap here, but return nicer error when fn is missing.
        let callee = self.get_function(&ident.value.0).unwrap();
        let args = arg_exprs.iter()
            .map(|e| self.compile_expr(&e.value).into())
            .collect::<Vec<BasicMetadataValueEnum>>();
        self.builder.build_call(callee, args.as_slice(), "tmp").try_as_basic_value()
    }

    // TODO: Make a result instead of unwrapping
    fn compile_expr(&mut self, expr: &Expression) -> BasicValueEnum<'ctx> {
        match expr {
            Expression::BitLiteral(b) => self.context.bool_type().const_int(if *b { 1 } else { 0 }, false).into(),
            Expression::NumberLiteral(n) => self.context.f64_type().const_float(*n).into(),
            Expression::QubitLiteral(q) =>
                self.builder.build_cast(
                    InstructionOpcode::IntToPtr, 
                    self.context.i64_type().const_int((*q).try_into().unwrap(), false),
                    self.qubit_type(),
                    "" // TODO: Not clear from inkwel or llvm docs what this argument does.
                ),
            Expression::Identifier(ident) => {
                // TODO: Don't unwrap here, but return nicer error when variable is missing.
                let alloca = self.variables.get(&ident.0).unwrap();
                self.builder.build_load(*alloca, "")
            },
            Expression::Call(ident, arg_exprs) => {
                let call = self.compile_call(ident, arg_exprs);
                // TODO: Don't unwrap here either, but turn into an actual error.
                call.left().unwrap_or_else(|| panic!("Function called as an expression, but does not have a return value.\n\tDebug info: {call:?}."))
            }
        }
    }

    // NB: Implicitly references fn_value_opt and variables for local
    //     symbol table.
    fn compile_body(&mut self, body: &Vec<Located<Statement>>) {
        for stmt in body.iter() {
            match &stmt.value {
                Statement::VariableDeclaration(ident, ty, rhs) => {
                    let alloca = self.create_entry_block_alloca(&ident.value.0, &ty.value);
                    self.builder.build_store(alloca, self.compile_expr(&rhs.value));
                    self.variables.insert(ident.value.0.to_string(), alloca);
                },
                Statement::Assignment(ident, rhs) => {
                    // TODO: Don't unwrap here.
                    let alloca = self.variables.get(&ident.value.0.to_string()).unwrap();
                    self.builder.build_store(*alloca, self.compile_expr(&rhs.value));
                },
                Statement::Call(ident, args) => {
                    // TODO: Don't ignore errors here.
                    self.compile_call(ident, args);
                },
                Statement::Return(expr) => {
                    let value = self.compile_expr(&expr.value);
                    self.builder.build_return(Some(&value));
                },
                Statement::If { condition, true_body, false_body} => {
                    let parent = self.fn_value();
                    let then_bb = self.context.append_basic_block(parent, "then");
                    let else_bb = self.context.append_basic_block(parent, "else");
                    let cont_bb = self.context.append_basic_block(parent, "ifcont");
                    let cond = self.compile_expr(&condition.value);
                    let cond = match cond {
                        BasicValueEnum::IntValue(cond) => cond,
                        // TODO: Don't unwrap here.
                        _ => panic!("Expected a boolean condition, but got {cond:?}")
                    };

                    self.builder.build_conditional_branch(cond, then_bb, else_bb);

                    // Build then block.
                    self.builder.position_at_end(then_bb);
                    self.compile_body(true_body);
                    self.builder.build_unconditional_branch(cont_bb);
                    let then_bb = self.builder.get_insert_block().unwrap();

                    // Built the else block.
                    self.builder.position_at_end(else_bb);
                    self.compile_body(false_body);
                    self.builder.build_unconditional_branch(cont_bb);
                    let else_bb = self.builder.get_insert_block().unwrap();

                    // NB: We don't have to worry about phi nodes here, since we
                    //     used pointer logic instead — that's less efficient, such that
                    //     in practice, you'll likely want to use phi nodes and
                    //     reason about values on the stack instead when doing
                    //     anything more practical.
                    self.builder.position_at_end(cont_bb);
                },
                _ => todo!("not yet implemented: {stmt:?}")
            }
        }
    }

    pub fn compile(&mut self) {
        // We start by making prototypes for each file element in the source.
        // This allows us to make sure we can always emit call instructions
        // later on in the compilation process, as the function declaration
        // will always exist.
        for file_element in &self.program.0 {
            let compiled_proto = self.compile_prototype(&match &file_element.value {
                FileElement::Declaration(proto) => proto,
                FileElement::Definition { body, prototype } => prototype
            }.value).unwrap(); // TODO: don't unwrap!
        }

        // Once we've made an initial pass to build prototypes, we can run a
        // second pass to add function bodies directly.
        for file_element in &self.program.0 {
            match &file_element.value {
                FileElement::Declaration(_) => (),
                FileElement::Definition { body, prototype } => {
                    // TODO: Move this this logic into a new method for compiling
                    //       function arg decls.
                    // TODO: Fix unwrap, return as error using ?.
                    let function = self.get_function(&prototype.value.name.value.0).unwrap();
                    self.fn_value_opt = Some(function);
                    let entry = self.context.append_basic_block(function, "entry");
                    self.builder.position_at_end(entry);

                    // Load arguments in as variables.
                    self.variables.reserve(prototype.value.arguments.len());


                    for (arg, proto_arg) in function.get_param_iter().zip(prototype.value.arguments.iter()) {
                        let arg_name = proto_arg.value.0.value.0.clone();
                        let alloca = self.create_entry_block_alloca(&arg_name, &proto_arg.value.1.value);

                        self.builder.build_store(alloca, arg);

                        self.variables.insert(arg_name, alloca);
                    }

                    // Now that we've loaded arguments, we can compile the
                    // body itself.
                    self.compile_body(&body);

                    // TODO: Build return.
                }
            }
        }
    }
}

pub fn compile(source_file: PathBuf) -> Result<()> {
    // TODO: Need some way of getting source as String here so that we can
    //       attach error messages.
    let program = build_ast(source_file)?;

    let context = Context::create();
    let module = context.create_module("qk");
    let builder = context.create_builder();

    // Initialize the pass manager.
    let fpm = PassManager::create(&module);
    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    fpm.add_gvn_pass();
    fpm.add_cfg_simplification_pass();
    fpm.add_basic_alias_analysis_pass();
    fpm.add_promote_memory_to_register_pass();
    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    fpm.initialize();

    let mut compiler = Compiler {
        builder: &builder,
        context: &context,
        fpm: &fpm,
        module: &module,
        program: &program,
        fn_value_opt: None,
        variables: HashMap::new()
    };

    compiler.compile();
    let ir = module.print_to_string().to_string();
    println!("Compiled IR:\n{ir}");

    Ok(())
}

pub fn run_compile_cmd(source_file: PathBuf) -> miette::Result<()> {
    Ok(compile(source_file)?)
}
