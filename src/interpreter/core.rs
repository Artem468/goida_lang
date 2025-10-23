use crate::ast::prelude::Program;
use crate::interpreter::structs::{Environment, Interpreter, Module, RuntimeError, Value};
use crate::interpreter::traits::{CoreOperations, InterpreterClasses, StatementExecutor};
use std::collections::HashMap;
use std::rc::Rc;
use crate::grammar;

impl CoreOperations for Interpreter {
    fn new(dir: std::path::PathBuf) -> Self {
        Interpreter {
            environment: Environment::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            modules: HashMap::new(),
            current_dir: dir,
            current_module: None,
        }
    }

    fn into_module(self, program: Program) -> Module {
        Module {
            functions: self.functions,
            classes: self.classes,
            environment: self.environment,
            program,
        }
    }

    fn interpret(&mut self, program: Program) -> Result<(), RuntimeError> {
        let module_name = program.arena.resolve_symbol(program.name).unwrap().to_string();
        self.current_module = Some(module_name.clone());

        for import in &program.imports {
            for path_symbol in &import.files {
                let path = program.arena.resolve_symbol(*path_symbol).unwrap();
                let relative_path = std::path::Path::new(path);
                let full_path = self.current_dir.join(relative_path).with_extension("goida");
                let file_stem = full_path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
                    RuntimeError::InvalidOperation(format!(
                        "Невозможно получить имя модуля из пути: {}",
                        full_path.display()
                    ))
                })?;
                let code = std::fs::read_to_string(&full_path).map_err(|err| {
                    RuntimeError::IOError(format!("{} | {err}", full_path.display()))
                })?;

                let mut program = Program::new(file_stem.to_string());
                let parser = grammar::ProgramParser::new();
                let _ = parser.parse(&mut program, code.as_str());

                let mut sub_interpreter = Interpreter::new(
                    full_path.parent().unwrap_or(&self.current_dir).to_path_buf(),
                );
                sub_interpreter.interpret(program.clone())?;
                self.modules.insert(file_stem.to_string(), sub_interpreter.into_module(program));
            }
        }

        for class_def in &program.classes {
            self.register_class(class_def, &program)?;
        }

        for function in &program.functions {
            let func_name = program.arena.resolve_symbol(function.name).unwrap().to_string();
            self.environment.define(func_name.clone(), Value::Function(Rc::new(function.clone())));
            self.functions.insert(func_name, function.clone());
        }

        for &stmt_id in &program.statements {
            match self.execute_statement(stmt_id, &program) {
                Err(RuntimeError::Return(_)) => {}
                Err(e) => return Err(e),
                Ok(()) => {}
            }
        }

        Ok(())
    }
}