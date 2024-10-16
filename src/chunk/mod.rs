use std::fmt::Debug;

use crate::{common::U8_COUNT_USIZE, value::Value};
mod debug;

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    Return,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Nil,
    True,
    False,
    Not,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    CloseUpvalue,
    Inherit,
    Call(u8),
    Constant(u8),
    Class(u8),
    DefineGlobal(u8),
    GetGlobal(u8),
    SetGlobal(u8),
    GetLocal(u8),
    SetLocal(u8),
    GetProperty(u8),
    SetProperty(u8),
    GetUpvalue(u8),
    SetUpvalue(u8),
    JumpIfFalse(u8),
    Jump(u8),
    Loop(u8),
    Closure(u8),
    ClosureIsLocalByte(bool),
    ClosureIndexByte(u8),
    Method(u8),
    Invoke(u8, u8),
    SuperInvoke(u8, u8),
    GetSuper(u8),
}

const CONSTANTS_DEFAULT: Option<Value> = None;

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Operation>,
    pub lines: Vec<usize>,
    pub constants: [Option<Value>; CONSTANTS_MAX],
    pub constants_count: usize,
}

const CONSTANTS_MAX: usize = U8_COUNT_USIZE;
impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: [CONSTANTS_DEFAULT; CONSTANTS_MAX],
            constants_count: 0,
        }
    }

    pub fn write(&mut self, operation: Operation, line: usize) {
        self.code.push(operation);
        self.lines.push(line);
    }

    /// Add constant to [`Chunk`], and return its index.
    /// Returns `None` if adding a new constant would overflow the constants stack.
    pub fn add_constant(&mut self, value: Value) -> Option<u8> {
        if self.constants_count == U8_COUNT_USIZE {
            return None;
        };
        let index = self.constants_count;
        self.constants[index] = Some(value);
        self.constants_count += 1;
        Some(index as u8)
    }
}
