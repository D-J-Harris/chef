use std::collections::VecDeque;

use crate::chunk::Chunk;
use crate::chunk::Operation;
use crate::compiler;
use crate::value::Value;
use crate::value::ValueOperationResult;
use crate::vm::InterpretResult::{CompileError, Ok, RuntimeError};

type RuntimeErrorLine = usize;

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError(RuntimeErrorLine, String),
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
        println!("==== Interpreting Chunk ====");
        for (offset, operation) in chunk.code.iter().enumerate() {
            if cfg!(feature = "debug-trace-execution") {
                println!("Value stack: {:?}", self.value_stack);
                let _ = chunk.disassemble_instruction(offset);
            }
            match operation {
                Operation::Return => {
                    println!("Returned {:?}", self.value_stack.pop_back());
                    return Ok;
                }
                Operation::Constant(index) => {
                    let Some(constant) = chunk.constants.get(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    self.value_stack.push_back(*constant);
                }
                Operation::Negate => {
                    let Some(constant) = self.value_stack.back_mut() else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    if constant.negate() == ValueOperationResult::Error {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Operand must be a number.".into(),
                        );
                    };
                }
                Operation::Add => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    if a.add(b) == ValueOperationResult::Error {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Operands must be numbers.".into(),
                        );
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Subtract => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    if a.sub(b) == ValueOperationResult::Error {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Operands must be numbers.".into(),
                        );
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Multiply => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    if a.mul(b) == ValueOperationResult::Error {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Operands must be numbers.".into(),
                        );
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Divide => {
                    let (Some(b), Some(mut a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    if a.div(b) == ValueOperationResult::Error {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Operands must be numbers.".into(),
                        );
                    };
                    self.value_stack.push_back(a);
                }
                Operation::Nil => self.value_stack.push_back(Value::Nil),
                Operation::True => self.value_stack.push_back(Value::Boolean(true)),
                Operation::False => self.value_stack.push_back(Value::Boolean(false)),
            }
        }
        RuntimeError(0, "Execution ended early.".into())
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        match compiler::compile(source) {
            Some(chunk) => self.run(&chunk),
            None => return CompileError,
        }
    }
}
