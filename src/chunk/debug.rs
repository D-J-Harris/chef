use crate::chunk::Operation;

use super::Chunk;

impl Chunk {
    pub fn disassemble(&self) {
        println!("====== Chunk Disassembly ======");
        let mut offset = 0;
        while offset < self.code.len() {
            match self.disassemble_instruction(offset) {
                Some(next_offset) => offset = next_offset,
                None => {
                    eprintln!("could not disassemble operation at offset {offset:04}");
                    break;
                }
            }
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> Option<usize> {
        let operation = unsafe { self.code.get_unchecked(offset) };
        let line = unsafe { self.lines.get_unchecked(offset) };
        if offset > 0 && line == unsafe { self.lines.get_unchecked(offset - 1) } {
            print!("{offset:0>4} {:>9}  ", "|");
        } else {
            print!("{offset:0>4} {line:>9}  ");
        }
        match operation {
            Operation::Return => Some(self.disassemble_simple_instruction(operation, offset)),
            Operation::Constant(index) => {
                self.disassemble_constant_instruction(operation, offset, *index as usize)
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
            | Operation::Less => Some(self.disassemble_simple_instruction(operation, offset)),
        }
    }

    fn disassemble_simple_instruction(&self, operation: &Operation, offset: usize) -> usize {
        println!("{operation:?}");
        offset + 1
    }

    fn disassemble_constant_instruction(
        &self,
        operation: &Operation,
        offset: usize,
        constant_index: usize,
    ) -> Option<usize> {
        let constant = self.constants.get(constant_index)?;
        println!("{operation:?} [constant: {constant:?}]");
        Some(offset + 1)
    }
}
