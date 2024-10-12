use std::fmt::Debug;

use crate::{common::U8_MAX, value::Value};
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
}

#[derive(Debug)]
pub struct Chunk {
    pub code: Vec<Operation>,
    pub lines: Vec<usize>,
    pub constants: [Option<Value>; U8_MAX],
    pub constants_count: usize,
}

const CONSTANT_DEFAULT: Option<Value> = None;
impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: [CONSTANT_DEFAULT; U8_MAX],
            constants_count: 0,
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
            let Some(Some(constant)) = self.constants.get(index) else {
                break;
            };
            match value.eq(constant) {
                true => return Some(index as u8),
                false => continue,
            }
        }
        if self.constants_count > U8_MAX {
            // TODO: propagate as error rather than Option
            return None;
        };
        self.constants[self.constants_count] = Some(value);
        self.constants_count += 1;
        Some(self.constants_count as u8 - 1)
    }
}
