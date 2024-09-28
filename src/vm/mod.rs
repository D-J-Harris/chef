use std::collections::VecDeque;

use crate::chunk::Chunk;
use crate::chunk::Operation;
use crate::value::Value;

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct Vm {
    value_stack: VecDeque<Value>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            value_stack: VecDeque::new(),
        }
    }

    pub fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        for (offset, operation) in chunk.code.iter().enumerate() {
            if cfg!(feature = "vm-trace") {
                let _ = chunk.disassemble_instruction(offset);
                println!("Value stack: {:?}", self.value_stack)
            }
            match operation {
                Operation::Return => {
                    println!("Returned {:?}", self.value_stack.pop_back());
                    return InterpretResult::Ok;
                }
                Operation::Constant(index) => {
                    let Some(constant) = chunk.constants.get(*index as usize) else {
                        return InterpretResult::RuntimeError;
                    };
                    self.value_stack.push_back(*constant);
                }
                Operation::Negation => {
                    let Some(constant) = self.value_stack.back_mut() else {
                        return InterpretResult::RuntimeError;
                    };
                    constant.negate();
                }
            }
        }
        InterpretResult::RuntimeError
    }
}
