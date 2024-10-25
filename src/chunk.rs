use std::fmt::Debug;

use gc_arena::Collect;

use crate::{common::CONSTANTS_MAX_COUNT, value::Value};

pub const RETURN: u8 = 1;
pub const NEGATE: u8 = 2;
pub const ADD: u8 = 3;
pub const SUBTRACT: u8 = 4;
pub const MULTIPLY: u8 = 5;
pub const DIVIDE: u8 = 6;
pub const NIL: u8 = 7;
pub const TRUE: u8 = 8;
pub const FALSE: u8 = 9;
pub const NOT: u8 = 10;
pub const EQUAL: u8 = 11;
pub const GREATER: u8 = 12;
pub const LESS: u8 = 13;
pub const PRINT: u8 = 14;
pub const POP: u8 = 15;
pub const CLOSE_UPVALUE: u8 = 16;
pub const INHERIT: u8 = 17;
pub const CALL: u8 = 18;
pub const CONSTANT: u8 = 19;
pub const CLASS: u8 = 20;
pub const DEFINE_GLOBAL: u8 = 21;
pub const GET_GLOBAL: u8 = 22;
pub const SET_GLOBAL: u8 = 23;
pub const GET_LOCAL: u8 = 24;
pub const SET_LOCAL: u8 = 25;
pub const GET_PROPERTY: u8 = 26;
pub const SET_PROPERTY: u8 = 27;
pub const GET_UPVALUE: u8 = 28;
pub const SET_UPVALUE: u8 = 29;
pub const JUMP_IF_FALSE: u8 = 30;
pub const JUMP: u8 = 31;
pub const LOOP: u8 = 32;
pub const CLOSURE: u8 = 33;
pub const METHOD: u8 = 34;
pub const INVOKE: u8 = 35;
pub const SUPER_INVOKE: u8 = 36;
pub const GET_SUPER: u8 = 37;

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
        let operation = self.code[offset];
        let line = self.lines[offset];
        if offset > 0 && line == self.lines[offset - 1] {
            print!("{offset:0>4} {:>9}  ", "|");
        } else {
            print!("{offset:0>4} {line:>9}  ");
        }
        match operation {
            CONSTANT | DEFINE_GLOBAL | GET_GLOBAL | SET_GLOBAL | GET_PROPERTY | SET_PROPERTY
            | CLOSURE | CLASS | METHOD | GET_SUPER => {
                self.disassemble_constant_instruction(operation, offset)
            }
            GET_LOCAL | SET_LOCAL | CALL | GET_UPVALUE | SET_UPVALUE => {
                self.disassemble_byte_instruction(operation, offset)
            }
            JUMP_IF_FALSE | JUMP | LOOP => self.disassemble_jump_instruction(operation, offset),
            NEGATE | ADD | SUBTRACT | MULTIPLY | DIVIDE | NIL | TRUE | FALSE | NOT | EQUAL
            | GREATER | LESS | PRINT | POP | RETURN | CLOSE_UPVALUE | INHERIT => {
                self.disassemble_simple_instruction(operation, offset)
            }
            INVOKE | SUPER_INVOKE => self.disassemble_invoke_instruction(operation, offset),
            _ => panic!("Invalid opcode."),
        }
    }

    fn disassemble_simple_instruction(&self, operation: u8, offset: usize) -> usize {
        println!("{operation:?}");
        offset + 1
    }

    fn disassemble_constant_instruction(&self, operation: u8, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        let constant = self.constants[constant_index];
        println!("{: <14} [constant: {constant}]", format!("{operation:?}"));
        offset + 2
    }

    fn disassemble_byte_instruction(&self, operation: u8, offset: usize) -> usize {
        let stack_index = self.code[offset + 1];
        println!(
            "{: <14} [stack_index: {stack_index}]",
            format!("{operation:?}")
        );
        offset + 2
    }

    fn disassemble_jump_instruction(&self, operation: u8, offset: usize) -> usize {
        let byte_1 = self.code[offset + 1];
        let byte_2 = self.code[offset + 2];
        let jump_offset = u16::from_le_bytes([byte_1, byte_2]);
        println!("{: <14} [offset: {jump_offset}]", format!("{operation:?}"));
        offset + 3
    }

    fn disassemble_invoke_instruction(&self, operation: u8, offset: usize) -> usize {
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
