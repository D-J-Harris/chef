use std::collections::VecDeque;

use crate::chunk::Chunk;
use crate::chunk::Operation;
use crate::value::Value;
use crate::value::ValueOperationResult;

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
                Operation::Negate => {
                    let Some(constant) = self.value_stack.back_mut() else {
                        return InterpretResult::RuntimeError;
                    };
                    if constant.negate() == ValueOperationResult::Error {
                        return InterpretResult::RuntimeError;
                    };
                }
                Operation::Add => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return InterpretResult::RuntimeError;
                    };
                    if a.add(b) == ValueOperationResult::Error {
                        return InterpretResult::RuntimeError;
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Subtract => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return InterpretResult::RuntimeError;
                    };
                    if a.sub(b) == ValueOperationResult::Error {
                        return InterpretResult::RuntimeError;
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Multiply => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return InterpretResult::RuntimeError;
                    };
                    if a.mul(b) == ValueOperationResult::Error {
                        return InterpretResult::RuntimeError;
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Divide => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return InterpretResult::RuntimeError;
                    };
                    if a.div(b) == ValueOperationResult::Error {
                        return InterpretResult::RuntimeError;
                    };
                    self.value_stack.push_back(a);
                }
            }
        }
        InterpretResult::RuntimeError
    }

    pub fn interpret(source: &str) -> InterpretResult {
        // compile()
        InterpretResult::Ok
    }
}
