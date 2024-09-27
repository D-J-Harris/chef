use std::fmt::Debug;

use crate::value::Value;
mod debug;

type ConstantIndex = usize;

#[derive(Debug)]
pub enum Operation {
    Return,
    Constant(ConstantIndex),
}

pub struct Chunk {
    code: Vec<Operation>,
    constants: Vec<Value>,
    lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, operation: Operation, line: u32) {
        self.code.push(operation);
        self.lines.push(line);
    }

    /// Add constant to [`Chunk`], and return its index.
    /// Returns previous index of constant, if it already exists in [`Chunk`].
    pub fn add_constant(&mut self, value: Value) -> usize {
        match self.constants.iter().position(|c| *c == value) {
            Some(index) => index,
            None => {
                self.constants.push(value);
                self.constants.len() - 1
            }
        }
    }
}
