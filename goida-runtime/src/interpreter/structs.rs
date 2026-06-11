use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::ast::prelude::{
    AstArena, ErrorData, ExprId, FunctionDefinition, Import, Parameter, Span, StmtId, Visibility,
};
use crate::ast::source::SourceManager;
use crate::bytecode::BytecodeModule;
use crate::hir::HirModule;
use crate::parser::structs::ParseError;
use crate::shared::SharedMut;
use libloading::Library;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{JoinHandle, ThreadId};
use string_interner::DefaultSymbol as Symbol;

pub type ThreadJoinState = Arc<Mutex<Option<JoinHandle<Result<(), RuntimeError>>>>>;
pub type BuiltinCallback =
    dyn Fn(&Interpreter, Vec<CallArgValue>, Span) -> Result<Value, RuntimeError> + Send + Sync;

#[derive(Clone, Debug)]
/// Runtime value representation used by the interpreter and built-ins.
pub enum Value {
    Number(i64),
    Float(f64),
    Pointer(usize),
    Text(String),
    Boolean(bool),
    Object(SharedMut<ClassInstance>),
    Class(SharedMut<RuntimeClassDefinition>),
    Function(Arc<FunctionDefinition>),
    Builtin(BuiltinFn),
    Module(Symbol),
    List(SharedMut<Vec<Value>>),
    Array(Arc<Vec<Value>>),
    Dict(SharedMut<HashMap<String, Value>>),
    Iterator(RuntimeIterator),
    Thread(RuntimeThread),
    Mutex(RuntimeMutex),
    RwLock(RuntimeRwLock),
    NativeResource(SharedMut<Box<dyn Any + Send + Sync>>),
    NativeGlobal(Arc<NativeGlobalBinding>),
    Empty,
}

#[derive(Clone, Debug)]
/// Lazy iterator pipeline over runtime values.
pub struct RuntimeIterator {
    pub source: Arc<Vec<Value>>,
    pub steps: Arc<Vec<IteratorStep>>,
}

impl RuntimeIterator {
    pub fn new(source: Arc<Vec<Value>>) -> Self {
        Self {
            source,
            steps: Arc::new(Vec::new()),
        }
    }

    pub fn with_step(&self, step: IteratorStep) -> Self {
        let mut steps = self.steps.as_ref().clone();
        steps.push(step);
        Self {
            source: self.source.clone(),
            steps: Arc::new(steps),
        }
    }
}

#[derive(Clone, Debug)]
/// Single lazy iterator transformation.
pub enum IteratorStep {
    Map(Value),
    Filter(Value),
}

#[derive(Clone, Debug)]
/// Join handle for a language-level background thread.
pub struct RuntimeThread {
    pub handle: ThreadJoinState,
}

impl RuntimeThread {
    /// Wraps a spawned Rust thread as a Goida runtime thread.
    pub fn new(handle: JoinHandle<Result<(), RuntimeError>>) -> Self {
        Self {
            handle: Arc::new(Mutex::new(Some(handle))),
        }
    }
}

#[derive(Clone, Debug)]
/// Runtime mutex value with reentrant ownership tracking.
pub struct RuntimeMutex {
    pub value: Arc<Mutex<Value>>,
    pub state: Arc<(Mutex<MutexLockState>, Condvar)>,
}

impl RuntimeMutex {
    /// Creates a mutex around an initial runtime value.
    pub fn new(value: Value) -> Self {
        Self {
            value: Arc::new(Mutex::new(value)),
            state: Arc::new((Mutex::new(MutexLockState::default()), Condvar::new())),
        }
    }
}

#[derive(Debug, Default)]
/// Ownership state for a runtime mutex.
pub struct MutexLockState {
    pub owner: Option<ThreadId>,
    pub depth: usize,
}

#[derive(Clone, Debug)]
/// Runtime read/write lock value with per-thread lock tracking.
pub struct RuntimeRwLock {
    pub value: Arc<RwLock<Value>>,
    pub state: Arc<(Mutex<RwLockState>, Condvar)>,
}

impl RuntimeRwLock {
    /// Creates a read/write lock around an initial runtime value.
    pub fn new(value: Value) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
            state: Arc::new((Mutex::new(RwLockState::default()), Condvar::new())),
        }
    }
}

#[derive(Debug, Default)]
/// Ownership state for a runtime read/write lock.
pub struct RwLockState {
    pub writer: Option<ThreadId>,
    pub writer_depth: usize,
    pub readers: HashMap<ThreadId, usize>,
}

#[derive(Clone, Debug)]
/// Binding to a function exported by a native dynamic library.
pub struct NativeFunctionBinding {
    pub module_id: Symbol,
    pub library_path: Arc<PathBuf>,
    pub symbol_name: Symbol,
    pub params: Vec<Parameter>,
    pub return_type: Option<u32>,
}

#[derive(Clone, Debug)]
/// Binding to a global exported by a native dynamic library.
pub struct NativeGlobalBinding {
    pub module_id: Symbol,
    pub library_path: Arc<PathBuf>,
    pub symbol_name: Symbol,
    pub value_type: u32,
}

#[derive(Debug)]
/// Loaded native dynamic library handle kept alive while bindings exist.
pub struct LoadedNativeLibrary {
    pub handle: Library,
}

#[derive(Clone, Debug)]
/// Runtime call argument with optional source-level name.
pub struct CallArgValue {
    pub name: Option<Symbol>,
    pub value: Value,
}

/// Convenience accessors for positional call arguments.
pub trait CallArgListExt {
    fn first_value(&self) -> Option<&Value>;
    fn get_value(&self, index: usize) -> Option<&Value>;
}

impl CallArgListExt for [CallArgValue] {
    fn first_value(&self) -> Option<&Value> {
        self.first().map(|arg| &arg.value)
    }

    fn get_value(&self, index: usize) -> Option<&Value> {
        self.get(index).map(|arg| &arg.value)
    }
}

impl CallArgListExt for Vec<CallArgValue> {
    fn first_value(&self) -> Option<&Value> {
        self.as_slice().first_value()
    }

    fn get_value(&self, index: usize) -> Option<&Value> {
        self.as_slice().get_value(index)
    }
}

#[derive(Clone)]
/// Native/built-in function callable from Goida code.
pub struct BuiltinFn(pub Arc<BuiltinCallback>);

#[derive(Clone, Debug)]
pub enum RuntimeMethodType {
    User(Arc<FunctionDefinition>),
    Native(Arc<BuiltinFn>),
}

#[derive(Clone, Debug)]
pub enum RuntimeFieldData {
    Expression(Option<ExprId>),
    Value(SharedMut<Value>),
}

#[derive(Clone, Debug)]
pub struct RuntimeClassDefinition {
    pub name: Symbol,
    pub base_class: Option<Symbol>,
    pub fields: HashMap<Symbol, (Visibility, bool, RuntimeFieldData)>,
    pub methods: HashMap<Symbol, (Visibility, bool, RuntimeMethodType)>,
    pub constructor: Option<RuntimeMethodType>,
    pub span: Span,
}

#[derive(Clone, Debug)]
/// Runtime object instance with per-instance field state.
pub struct ClassInstance {
    pub class_name: Symbol,
    pub fields: HashMap<Symbol, Option<crate::ast::prelude::ExprId>>,
    pub field_values: HashMap<Symbol, Value>,
    pub class_ref: SharedMut<RuntimeClassDefinition>,
}

#[derive(Debug)]
/// Runtime failures surfaced by the interpreter.
pub enum RuntimeError {
    UndefinedVariable(ErrorData),
    UndefinedFunction(ErrorData),
    UndefinedMethod(ErrorData),
    TypeMismatch(ErrorData),
    DivisionByZero(ErrorData),
    InvalidOperation(ErrorData),
    Return(ErrorData, Value),
    TypeError(ErrorData),
    IOError(ErrorData),
    ImportError(ParseError),
    Panic(ErrorData),
    Raised(ErrorData, String),
}

impl RuntimeError {
    /// Returns the Goida error class name corresponding to this runtime error.
    pub fn error_class_name(&self) -> String {
        match self {
            RuntimeError::UndefinedVariable(_) => "ОшибкаПеременной".to_string(),
            RuntimeError::UndefinedFunction(_) => "ОшибкаФункции".to_string(),
            RuntimeError::UndefinedMethod(_) => "ОшибкаМетода".to_string(),
            RuntimeError::TypeMismatch(_) | RuntimeError::TypeError(_) => "ОшибкаТипа".to_string(),
            RuntimeError::DivisionByZero(_) => "ОшибкаДеленияНаНоль".to_string(),
            RuntimeError::InvalidOperation(_) => "ОшибкаОперации".to_string(),
            RuntimeError::IOError(_) => "ОшибкаВводаВывода".to_string(),
            RuntimeError::ImportError(_) => "ОшибкаИмпорта".to_string(),
            RuntimeError::Panic(_) => "Паника".to_string(),
            RuntimeError::Raised(_, class_name) => class_name.clone(),
            RuntimeError::Return(..) => "Возврат".to_string(),
        }
    }

    /// Returns a human-readable message for diagnostics and catch bindings.
    pub fn error_message(&self) -> String {
        match self {
            RuntimeError::UndefinedVariable(err)
            | RuntimeError::UndefinedFunction(err)
            | RuntimeError::UndefinedMethod(err)
            | RuntimeError::TypeMismatch(err)
            | RuntimeError::DivisionByZero(err)
            | RuntimeError::InvalidOperation(err)
            | RuntimeError::TypeError(err)
            | RuntimeError::IOError(err)
            | RuntimeError::Panic(err)
            | RuntimeError::Raised(err, _) => err.message.clone(),
            RuntimeError::ImportError(err) => match err {
                ParseError::TypeError(err)
                | ParseError::InvalidSyntax(err)
                | ParseError::ImportError(err) => err.message.clone(),
            },
            RuntimeError::Return(err, value) => {
                if err.message.is_empty() {
                    value.to_string()
                } else {
                    err.message.clone()
                }
            }
        }
    }

    pub fn add_stack_frame(&mut self, name: impl Into<String>, location: Span) {
        match self {
            RuntimeError::UndefinedVariable(err)
            | RuntimeError::UndefinedFunction(err)
            | RuntimeError::UndefinedMethod(err)
            | RuntimeError::TypeMismatch(err)
            | RuntimeError::DivisionByZero(err)
            | RuntimeError::InvalidOperation(err)
            | RuntimeError::Return(err, _)
            | RuntimeError::TypeError(err)
            | RuntimeError::IOError(err)
            | RuntimeError::Panic(err)
            | RuntimeError::Raised(err, _) => err.push_frame(name, location),
            RuntimeError::ImportError(_) => {}
        }
    }
}

#[derive(Debug)]
/// Lexical environment frame.
pub struct Environment {
    /// Values owned by this lexical frame and addressed by `LocalSlot`.
    pub(crate) slots: Vec<SharedMut<Value>>,
    /// Direct references captured from visible parent bindings.
    pub(crate) upvalues: Vec<SharedMut<Value>>,
    /// Name resolution table; values themselves are never stored here.
    pub(crate) bindings: HashMap<Symbol, VariableSlot>,
    pub(crate) constants: HashSet<VariableSlot>,
    pub(crate) parent: Option<SharedMut<Environment>>,
    pub(crate) is_function: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// Numeric location produced by runtime name resolution.
pub enum VariableSlot {
    LocalSlot(u32),
    GlobalSlot(u32),
    UpvalueSlot(u32),
}

/// Shared string interner used by parser, arena and interpreter.
pub type SharedInterner = goida_model::SharedInterner;

#[derive(Debug)]
/// Main interpreter state.
pub struct Interpreter {
    pub(crate) std_classes: HashMap<Symbol, SharedMut<RuntimeClassDefinition>>,
    pub(crate) builtins: HashMap<Symbol, BuiltinFn>,
    pub modules: HashMap<Symbol, Module>,
    pub(crate) native_libraries: HashMap<PathBuf, SharedMut<LoadedNativeLibrary>>,
    pub interner: SharedInterner,
    pub(crate) environment: SharedMut<Environment>,
    pub(crate) background_threads: Vec<RuntimeThread>,
    pub(crate) method_depth: usize,
    pub(crate) heap: Arc<crate::interpreter::heap::ObjectHeap>,
    pub source_manager: SourceManager,
}

#[derive(Clone, Debug)]
/// Immutable compiler artifacts retained for tooling and execution.
pub struct CompiledModule {
    pub arena: AstArena,
    pub hir: HirModule,
    pub bytecode: BytecodeModule,
    pub functions: HashMap<Symbol, Arc<FunctionDefinition>>,
    pub body: Vec<StmtId>,
    pub imports: Vec<Import>,
}

#[derive(Clone, Debug)]
/// Compiled module identity plus mutable runtime declarations and globals.
pub struct Module {
    pub name: Symbol,
    pub path: PathBuf,
    pub compiled: CompiledModule,
    pub classes: HashMap<Symbol, SharedMut<RuntimeClassDefinition>>,
    pub modules: HashMap<Symbol, Module>,
    pub globals: HashMap<Symbol, Value>,
    pub(crate) global_slots: Vec<Option<SharedMut<Value>>>,
}

impl Deref for Module {
    type Target = CompiledModule;

    fn deref(&self) -> &Self::Target {
        &self.compiled
    }
}

impl DerefMut for Module {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.compiled
    }
}
