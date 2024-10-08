use std::fmt::Debug;

use crate::value::Value;
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
    Constant(u8),
    DefineGlobal(u8),
    GetGlobal(u8),
    SetGlobal(u8),
    GetLocal(u8),
    SetLocal(u8),
    JumpIfFalse(u8),
    Jump(u8),
    Loop(u8),
}

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Operation>,
    pub constants: [Value; u8::MAX as usize],
    pub constants_count: usize,
    pub lines: Vec<usize>,
}

const ARRAY_REPEAT_VALUE: Value = Value::Uninit;
impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: [ARRAY_REPEAT_VALUE; u8::MAX as usize],
            constants_count: 0,
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, operation: Operation, line: usize) {
        self.code.push(operation);
        self.lines.push(line);
    }

    /// Add constant to [`Chunk`], and return its index, otherwise return None.
    /// If it already exists in [`Chunk`], returns previous index of constant.
    pub fn add_constant(&mut self, value: Value) -> Option<u8> {
        for index in 0..self.constants_count {
            let Some(constant) = self.constants.get(index) else {
                break;
            };
            match value.eq(constant) {
                true => return Some(index as u8),
                false => continue,
            }
        }
        if self.constants_count >= u8::MAX as usize {
            return None;
        };
        self.constants[self.constants_count] = value;
        self.constants_count += 1;
        Some(self.constants_count as u8 - 1)
    }
}
