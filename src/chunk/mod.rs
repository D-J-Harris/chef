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
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn write(&mut self, operation: Operation) {
        self.code.push(operation);
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
