use crate::ast::prelude::{
    BinaryOperator, ClassDefinition, ExprId, FunctionDefinition, LiteralValue,
    NativeLibraryDefinition, Span, StmtId, UnaryOperator,
};
use crate::hir::{Binding, MethodResolution};
use std::collections::HashMap;
use std::sync::Arc;
use string_interner::DefaultSymbol as Symbol;

pub type Register = u32;

#[derive(Clone, Copy, Debug)]
pub struct RegisterArg {
    pub name: Option<Symbol>,
    pub register: Register,
}
#[derive(Clone, Debug)]
pub struct BytecodeHandler {
    pub error_type: Option<Symbol>,
    pub error_text: Option<Symbol>,
    pub body: Arc<Chunk>,
}

#[derive(Clone, Debug)]
pub enum Instruction {
    LoadLiteral {
        dst: Register,
        value: LiteralValue,
    },
    LoadName {
        dst: Register,
        name: Symbol,
        binding: Binding,
    },
    Unary {
        dst: Register,
        op: UnaryOperator,
        operand: Register,
    },
    Binary {
        dst: Register,
        op: BinaryOperator,
        left: Register,
        right: Register,
    },
    ToBoolean {
        dst: Register,
        source: Register,
    },
    CallDirect {
        dst: Register,
        name: Symbol,
        args: Vec<RegisterArg>,
    },
    Call {
        dst: Register,
        callable: Register,
        args: Vec<RegisterArg>,
    },
    ReadIndex {
        dst: Register,
        object: Register,
        index: Register,
    },
    ReadProperty {
        dst: Register,
        object: Register,
        property: Symbol,
        receiver_is_this: bool,
        receiver_name: Option<Symbol>,
    },
    CallMethod {
        dst: Register,
        object: Register,
        resolution: MethodResolution,
        args: Vec<RegisterArg>,
        receiver_is_this: bool,
    },
    NewObject {
        dst: Register,
        class_name: Symbol,
        args: Vec<RegisterArg>,
    },
    MakeLambda {
        dst: Register,
        function: FunctionDefinition,
    },
    InvalidThis {
        dst: Register,
    },
    StoreName {
        name: Symbol,
        binding: Binding,
        is_const: bool,
        source: Register,
    },
    StoreIndex {
        object: Register,
        index: Register,
        source: Register,
    },
    StoreProperty {
        object: Register,
        property: Symbol,
        source: Register,
        receiver_is_this: bool,
    },
    Jump(usize),
    JumpIfFalse {
        condition: Register,
        target: usize,
    },
    Scope(Arc<Chunk>),
    ForEach {
        variable: Symbol,
        iterable: Register,
        body: Arc<Chunk>,
    },
    Thread(Arc<Chunk>),
    Try {
        body: Arc<Chunk>,
        handlers: Vec<BytecodeHandler>,
    },
    Raise {
        error_type: Symbol,
        message: Option<Register>,
    },
    Return(Option<Register>),
    DefineFunction(FunctionDefinition),
    LoadNativeLibrary(NativeLibraryDefinition),
    DefineClass(ClassDefinition),
    Halt,
}

#[derive(Clone, Debug, Default)]
pub struct Chunk {
    pub code: Vec<Instruction>,
    pub spans: Vec<Span>,
    pub register_count: u32,
    pub result: Option<Register>,
}

impl Chunk {
    pub(super) fn emit(&mut self, instruction: Instruction, span: Span) -> usize {
        let address = self.code.len();
        self.code.push(instruction);
        self.spans.push(span);
        address
    }
}

#[derive(Clone, Debug, Default)]
pub struct BytecodeModule {
    pub module: Arc<Chunk>,
    pub bodies: HashMap<StmtId, Arc<Chunk>>,
    pub expressions: HashMap<ExprId, Arc<Chunk>>,
}
