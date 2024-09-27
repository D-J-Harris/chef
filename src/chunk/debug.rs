use crate::chunk::Operation;

use super::Chunk;

impl Chunk {
    pub fn disassemble(&self) {
        println!("== chunk ==");
        let mut offset = 0;
        while offset < self.code.len() {
            let operation = unsafe { self.code.get_unchecked(offset) };
            match self.disassemble_instruction(&operation, offset) {
                Some(next_offset) => offset = next_offset,
                None => {
                    eprintln!(
                        "could not disassemble operation {operation:?} at offset {offset:04}"
                    );
                    break;
                }
            }
        }
    }

    fn disassemble_instruction(&self, operation: &Operation, offset: usize) -> Option<usize> {
        match operation {
            Operation::Return => Some(self.disassemble_simple_instruction(operation, offset)),
            Operation::Constant(index) => {
                self.disassemble_constant_instruction(operation, offset, *index)
            }
        }
    }

    fn disassemble_simple_instruction(&self, operation: &Operation, offset: usize) -> usize {
        println!("{offset:04} {operation:?}");
        offset + 1
    }

    fn disassemble_constant_instruction(
        &self,
        operation: &Operation,
        offset: usize,
        constant_index: usize,
    ) -> Option<usize> {
        let constant = self.constants.get(constant_index)?;
        println!("{offset:04} {operation:?} {constant:?}");
        Some(offset + 1)
    }
}
