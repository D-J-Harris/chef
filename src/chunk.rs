use std::{fmt::Debug, mem::transmute};

use crate::{common::CONSTANTS_MAX_COUNT, value::Value};

#[derive(Debug)]
pub enum Opcode {
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
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    Constant,
    JumpIfFalse,
    Jump,
    Loop,
    Call,
    Function,
}

#[derive(Debug)]
pub struct Code {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: [Value; CONSTANTS_MAX_COUNT],
    pub constants_count: usize,
}

const ARRAY_REPEAT_VALUE: Value = Value::Nil;
impl Code {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            lines: Vec::new(),
            constants: [ARRAY_REPEAT_VALUE; CONSTANTS_MAX_COUNT],
            constants_count: 0,
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> Option<u8> {
        if self.constants_count == CONSTANTS_MAX_COUNT {
            return None;
        }
        self.constants[self.constants_count] = value;
        self.constants_count += 1;
        Some((self.constants_count - 1) as u8)
    }
}

#[allow(unused)]
impl Code {
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
        let operation: Opcode = unsafe { transmute(byte) };
        match operation {
            Opcode::Return => self.disassemble_simple_instruction(operation, offset),
            Opcode::Negate => self.disassemble_simple_instruction(operation, offset),
            Opcode::Add => self.disassemble_simple_instruction(operation, offset),
            Opcode::Subtract => self.disassemble_simple_instruction(operation, offset),
            Opcode::Multiply => self.disassemble_simple_instruction(operation, offset),
            Opcode::Divide => self.disassemble_simple_instruction(operation, offset),
            Opcode::Nil => self.disassemble_simple_instruction(operation, offset),
            Opcode::True => self.disassemble_simple_instruction(operation, offset),
            Opcode::False => self.disassemble_simple_instruction(operation, offset),
            Opcode::Not => self.disassemble_simple_instruction(operation, offset),
            Opcode::Equal => self.disassemble_simple_instruction(operation, offset),
            Opcode::Greater => self.disassemble_simple_instruction(operation, offset),
            Opcode::Less => self.disassemble_simple_instruction(operation, offset),
            Opcode::Print => self.disassemble_simple_instruction(operation, offset),
            Opcode::Pop => self.disassemble_simple_instruction(operation, offset),
            Opcode::Function => self.disassemble_constant_instruction(operation, offset),
            Opcode::DefineGlobal => self.disassemble_constant_instruction(operation, offset),
            Opcode::GetGlobal => self.disassemble_constant_instruction(operation, offset),
            Opcode::SetGlobal => self.disassemble_constant_instruction(operation, offset),
            Opcode::GetLocal => self.disassemble_byte_instruction(operation, offset),
            Opcode::SetLocal => self.disassemble_byte_instruction(operation, offset),
            Opcode::Constant => self.disassemble_constant_instruction(operation, offset),
            Opcode::JumpIfFalse => self.disassemble_jump_instruction(operation, offset),
            Opcode::Jump => self.disassemble_jump_instruction(operation, offset),
            Opcode::Loop => self.disassemble_jump_instruction(operation, offset),
            Opcode::Call => self.disassemble_byte_instruction(operation, offset),
        }
    }

    fn disassemble_simple_instruction(&self, operation: Opcode, offset: usize) -> usize {
        println!("{operation:?}");
        offset + 1
    }

    fn disassemble_constant_instruction(&self, operation: Opcode, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        let constant = &self.constants[constant_index];
        println!("{: <14} [constant: {constant}]", format!("{operation:?}"));
        offset + 2
    }

    fn disassemble_byte_instruction(&self, operation: Opcode, offset: usize) -> usize {
        let stack_index = self.code[offset + 1];
        println!(
            "{: <14} [stack_index: {stack_index}]",
            format!("{operation:?}")
        );
        offset + 2
    }

    fn disassemble_jump_instruction(&self, operation: Opcode, offset: usize) -> usize {
        let byte_1 = self.code[offset + 1];
        let byte_2 = self.code[offset + 2];
        let jump_offset = u16::from_le_bytes([byte_1, byte_2]);
        println!("{: <14} [offset: {jump_offset}]", format!("{operation:?}"));
        offset + 3
    }

    fn disassemble_invoke_instruction(&self, operation: Opcode, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        let argument_count = self.code[offset + 2];
        let constant = &self.constants[constant_index];
        println!(
            "{: <14} [args: {argument_count}, constant: {constant}]",
            format!("{operation:?}")
        );
        offset + 3
    }
}
