use std::collections::HashMap;
use std::collections::VecDeque;

use crate::chunk::Chunk;
use crate::chunk::Operation;
use crate::compiler;
use crate::value::Value;
use crate::vm::InterpretResult::{CompileError, Ok as InterpretOk, RuntimeError};

type RuntimeErrorLine = usize;

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError(RuntimeErrorLine, String),
}

pub struct Vm {
    value_stack: VecDeque<Value>,
    global_identifiers: HashMap<String, Value>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            value_stack: VecDeque::new(),
            global_identifiers: HashMap::new(),
        }
    }

    pub fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        if cfg!(feature = "debug-trace-execution") {
            println!("==== Interpreting Chunk ====");
        }
        for (offset, operation) in chunk.code.iter().enumerate() {
            if cfg!(feature = "debug-trace-execution") {
                println!("Value stack: {:?}", self.value_stack);
                let _ = chunk.disassemble_instruction(offset);
            }
            match operation {
                Operation::Return => {
                    return InterpretOk;
                }
                Operation::Constant(index) => {
                    let Some(constant) = chunk.constants.get(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    self.value_stack.push_back(constant.clone()); // TODO: remove Clone
                }
                Operation::Negate => {
                    let Some(constant) = self.value_stack.back_mut() else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    if let Err(e) = constant.negate() {
                        return RuntimeError(chunk.lines[offset], e);
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
                    match a.add_assign(b) {
                        Ok(()) => self.value_stack.push_back(a),
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
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
                    match a.sub_assign(b) {
                        Ok(()) => self.value_stack.push_back(a),
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
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
                    match a.mul_assign(b) {
                        Ok(()) => self.value_stack.push_back(a),
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
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
                    match a.div_assign(b) {
                        Ok(()) => self.value_stack.push_back(a),
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
                }
                Operation::Nil => self.value_stack.push_back(Value::Nil),
                Operation::True => self.value_stack.push_back(Value::Boolean(true)),
                Operation::False => self.value_stack.push_back(Value::Boolean(false)),
                Operation::Not => {
                    let Some(constant) = self.value_stack.pop_back() else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    let result = match constant.falsey() {
                        Ok(b) => b,
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
                    self.value_stack.push_back(Value::Boolean(result))
                }
                Operation::Equal => {
                    let (Some(b), Some(a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    let result = a.is_equal(b);
                    self.value_stack.push_back(Value::Boolean(result))
                }
                Operation::Greater => {
                    let (Some(b), Some(a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    match a.is_greater(b) {
                        Ok(result) => self.value_stack.push_back(Value::Boolean(result)),
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
                }
                Operation::Less => {
                    let (Some(b), Some(a)) =
                        (self.value_stack.pop_back(), self.value_stack.pop_back())
                    else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "Not enough constants initialised.".into(),
                        );
                    };
                    match a.is_less(b) {
                        Ok(result) => self.value_stack.push_back(Value::Boolean(result)),
                        Err(e) => return RuntimeError(chunk.lines[offset], e),
                    };
                }
                Operation::Print => {
                    let Some(constant) = self.value_stack.pop_back() else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    println!("{constant}");
                }
                Operation::Pop => drop(self.value_stack.pop_back()),
                Operation::DefineGlobal(index) => {
                    let Some(Value::String(name)) = chunk.constants.get(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No variable initialised with this name.".into(),
                        );
                    };
                    let Some(constant) = self.value_stack.pop_back() else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    self.global_identifiers.insert(name.to_owned(), constant);
                }
                Operation::GetGlobal(index) => {
                    let Some(Value::String(name)) = chunk.constants.get(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No variable initialised with this name.".into(),
                        );
                    };
                    let Some(constant) = self.global_identifiers.get(name) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            format!("No constant initialised with name '{name}'."),
                        );
                    };
                    self.value_stack.push_back(constant.clone()); // TODO: remove Clone
                }
                Operation::SetGlobal(index) => {
                    let Some(Value::String(name)) = chunk.constants.get(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No variable initialised with this name.".into(),
                        );
                    };
                    let Some(constant) = self.value_stack.pop_back() else {
                        return RuntimeError(
                            chunk.lines[offset],
                            "No constants initialised.".into(),
                        );
                    };
                    self.global_identifiers.insert(name.to_owned(), constant);
                }
                Operation::GetLocal(index) => {
                    let Some(value) = self.value_stack.get(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            format!("No value at index '{index}' in stack."),
                        );
                    };
                    self.value_stack.push_back(value.clone());
                }
                Operation::SetLocal(index) => {
                    let Some(replacement_value) = self.value_stack.back() else {
                        return RuntimeError(chunk.lines[offset], format!("No values in stack."));
                    };
                    let replacement_value = replacement_value.clone();
                    let Some(mut_value) = self.value_stack.get_mut(*index as usize) else {
                        return RuntimeError(
                            chunk.lines[offset],
                            format!("No value at index '{index}' in stack."),
                        );
                    };
                    *mut_value = replacement_value.clone()
                }
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
