use std::fmt::Debug;

use crate::value::Value;
mod debug;

type ConstantIndex = u8;

// TODO: remove Clone
#[derive(Debug, Clone)]
pub enum Operation {
    Return,
    Constant(ConstantIndex),
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
}

#[derive(Clone)]
pub struct Chunk {
    pub code: Vec<Operation>,
    pub constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, operation: Operation, line: usize) {
        self.code.push(operation);
        self.lines.push(line);
    }

    /// Add constant to [`Chunk`], and return its index.
    /// Returns previous index of constant, if it already exists in [`Chunk`].
    ///
    /// Due to indexing constants using [`u8`], return None if [`u8::MAX`]
    /// constants are added to the chunk already.
    pub fn add_constant(&mut self, value: Value) -> Option<u8> {
        let constant_index = match self.constants.iter().position(|c| *c == value) {
            Some(index) => index,
            None => match self.constants.len() >= u8::MAX.into() {
                true => return None,
                false => {
                    self.constants.push(value);
                    self.constants.len() - 1
                }
            },
        };
        // Safety: we ensure this index <= u8::MAX
        Some(constant_index as u8)
    }
}
