use std::fmt::Debug;

use crate::{common::U8_COUNT_USIZE, value::Value};

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

#[cfg(feature = "debug_trace")]
impl Chunk {
    pub fn disassemble(&self, name: &str) {
        println!("====== Chunk {name} ======");
        for offset in 0..self.code.len() - 1 {
            self.disassemble_instruction(offset)
        }
        println!();
    }

    pub fn disassemble_instruction(&self, offset: usize) {
        let operation = self.code[offset];
        let line = self.lines[offset];
        if offset > 0 && line == self.lines[offset - 1] {
            print!("{offset:0>4} {:>9}  ", "|");
        } else {
            print!("{offset:0>4} {line:>9}  ");
        }
        match operation {
            Operation::Constant(index)
            | Operation::DefineGlobal(index)
            | Operation::GetGlobal(index)
            | Operation::SetGlobal(index)
            | Operation::GetProperty(index)
            | Operation::SetProperty(index)
            | Operation::Closure(index)
            | Operation::Class(index)
            | Operation::Method(index)
            | Operation::GetSuper(index) => {
                self.disassemble_constant_instruction(operation, index as usize)
            }
            Operation::GetLocal(slot_value)
            | Operation::SetLocal(slot_value)
            | Operation::Call(slot_value)
            | Operation::GetUpvalue(slot_value)
            | Operation::SetUpvalue(slot_value) => {
                self.disassemble_byte_instruction(operation, slot_value as usize)
            }
            Operation::JumpIfFalse(jump) | Operation::Jump(jump) => {
                self.disassemble_jump_instruction(operation, jump, false)
            }
            Operation::Loop(jump) => self.disassemble_jump_instruction(operation, jump, true),
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
            | Operation::Inherit => self.disassemble_simple_instruction(operation),
            Operation::ClosureIsLocalByte(is_local) => match is_local {
                true => println!("Local Value:"),
                false => println!("Upvalue:"),
            },
            Operation::ClosureIndexByte(index) => println!("Index {}", index),
            Operation::Invoke(index, argument_count)
            | Operation::SuperInvoke(index, argument_count) => {
                self.disassemble_invoke_instruction(index as usize, argument_count)
            }
        }
    }

    fn disassemble_simple_instruction(&self, operation: Operation) {
        println!("{operation:?}");
    }

    fn disassemble_constant_instruction(&self, operation: Operation, constant_index: usize) {
        let constant = self
            .constants
            .get(constant_index)
            .unwrap()
            .as_ref()
            .unwrap();
        println!("{operation:?} [constant: {constant}]");
    }

    fn disassemble_byte_instruction(&self, operation: Operation, slot_value: usize) {
        println!("{operation:?} [slot: {slot_value}]");
    }

    fn disassemble_jump_instruction(
        &self,
        operation: Operation,
        jump: u8,
        is_jump_backwards: bool,
    ) {
        println!(
            "{operation:?} [jump: {}, backwards: {}]",
            jump, is_jump_backwards
        );
    }

    fn disassemble_invoke_instruction(&self, index: usize, argument_count: u8) {
        let constant = self.constants.get(index).unwrap().as_ref().unwrap();
        println!("Invoke ({argument_count} args) [constant: {constant}]");
    }
}
