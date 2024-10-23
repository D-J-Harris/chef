use std::fmt::Debug;

use gc_arena::Collect;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{common::CONSTANTS_MAX_COUNT, value::Value};

#[derive(Debug, Copy, Clone, Collect, FromPrimitive)]
#[collect(require_static)]
#[repr(u8)]
pub enum Operation {
    Return = 1,
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
    Call,
    Constant,
    Class,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    GetProperty,
    SetProperty,
    GetUpvalue,
    SetUpvalue,
    JumpIfFalse,
    Jump,
    Loop,
    Closure,
    Method,
    Invoke,
    SuperInvoke,
    GetSuper,
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct Chunk<'gc> {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: [Value<'gc>; CONSTANTS_MAX_COUNT],
    pub constants_count: usize,
}

impl<'gc> Chunk<'gc> {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: [Value::Nil; CONSTANTS_MAX_COUNT],
            constants_count: 0,
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    /// Add constant to [`Chunk`], and return its index.
    /// Returns `None` if adding a new constant would overflow the constants stack.
    pub fn add_constant(&mut self, value: Value<'gc>) -> Option<u8> {
        if self.constants_count == CONSTANTS_MAX_COUNT {
            return None;
        }
        self.constants[self.constants_count] = value;
        self.constants_count += 1;
        Some((self.constants_count - 1) as u8)
    }
}

#[allow(unused)]
impl Chunk<'_> {
    pub fn disassemble(&self, name: &str) {
        println!("====== Chunk {name} ======");
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset)
        }
        println!();
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        let byte = self.code[offset];
        let line = self.lines[offset];
        if offset > 0 && line == self.lines[offset - 1] {
            print!("{offset:0>4} {:>9}  ", "|");
        } else {
            print!("{offset:0>4} {line:>9}  ");
        }
        let operation = Operation::from_u8(byte).expect("Invalid opcode.");
        match operation {
            Operation::Constant
            | Operation::DefineGlobal
            | Operation::GetGlobal
            | Operation::SetGlobal
            | Operation::GetProperty
            | Operation::SetProperty
            | Operation::Closure
            | Operation::Class
            | Operation::Method
            | Operation::GetSuper => self.disassemble_constant_instruction(operation, offset),
            Operation::GetLocal
            | Operation::SetLocal
            | Operation::Call
            | Operation::GetUpvalue
            | Operation::SetUpvalue => self.disassemble_byte_instruction(operation, offset),
            Operation::JumpIfFalse | Operation::Jump | Operation::Loop => {
                self.disassemble_jump_instruction(operation, offset)
            }
            Operation::Negate
            | Operation::Add
            | Operation::Subtract
            | Operation::Multiply
            | Operation::Divide
            | Operation::Nil
            | Operation::True
            | Operation::False
            | Operation::Not
            | Operation::Equal
            | Operation::Greater
            | Operation::Less
            | Operation::Print
            | Operation::Pop
            | Operation::Return
            | Operation::CloseUpvalue
            | Operation::Inherit => self.disassemble_simple_instruction(operation, offset),
            Operation::Invoke | Operation::SuperInvoke => {
                self.disassemble_invoke_instruction(operation, offset)
            }
        }
    }

    fn disassemble_simple_instruction(&self, operation: Operation, offset: usize) -> usize {
        println!("{operation:?}");
        offset + 1
    }

    fn disassemble_constant_instruction(&self, operation: Operation, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        let constant = self.constants[constant_index];
        println!("{: <14} [constant: {constant}]", format!("{operation:?}"));
        offset + 2
    }

    fn disassemble_byte_instruction(&self, operation: Operation, offset: usize) -> usize {
        let stack_index = self.code[offset + 1];
        println!(
            "{: <14} [stack_index: {stack_index}]",
            format!("{operation:?}")
        );
        offset + 2
    }

    fn disassemble_jump_instruction(&self, operation: Operation, offset: usize) -> usize {
        let byte_1 = self.code[offset + 1];
        let byte_2 = self.code[offset + 2];
        let jump_offset = u16::from_le_bytes([byte_1, byte_2]);
        println!("{: <14} [offset: {jump_offset}]", format!("{operation:?}"));
        offset + 3
    }

    fn disassemble_invoke_instruction(&self, operation: Operation, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        let argument_count = self.code[offset + 2];
        let constant = self.constants[constant_index];
        println!(
            "{: <14} [args: {argument_count}, constant: {constant}]",
            format!("{operation:?}")
        );
        offset + 3
    }
}
