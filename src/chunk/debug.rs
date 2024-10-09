use crate::chunk::Operation;

use super::Chunk;

impl Chunk {
    #[cfg(feature = "debug_print_code")]
    pub fn disassemble(&self, name: &str) {
        println!("====== Chunk {name} ======");
        for offset in 0..self.code.len() - 1 {
            self.disassemble_instruction(offset)
        }
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
            | Operation::SetGlobal(index) => {
                self.disassemble_constant_instruction(operation, index as usize)
            }
            Operation::GetLocal(index) | Operation::SetLocal(index) => {
                self.disassemble_byte_instruction(operation, index as usize)
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
            | Operation::Return => self.disassemble_simple_instruction(operation),
        }
    }

    fn disassemble_simple_instruction(&self, operation: Operation) {
        println!("{operation:?}");
    }

    fn disassemble_constant_instruction(&self, operation: Operation, constant_index: usize) {
        let constant = self.constants.get(constant_index).unwrap();
        println!("{operation:?} [constant: {constant}]");
    }

    fn disassemble_byte_instruction(&self, operation: Operation, slot_index: usize) {
        println!("{operation:?} [slot: {slot_index}]");
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
}
