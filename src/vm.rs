use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::rc::Rc;

use crate::chunk::{Chunk, Operation};
use crate::compiler;
use crate::objects::{Function, Object};
use crate::value::Value;
use crate::vm::InterpretResult::{CompileError, Ok as InterpretOk, RuntimeError};

const FRAMES_MAX: usize = 32;
const STACK_MAX: usize = u8::MAX as usize;

type RuntimeErrorLine = usize;

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError(RuntimeErrorLine, String),
}

struct CallFrame {
    function: Rc<Function>,
    start_slot: usize,
    ip: usize,
}

impl CallFrame {
    fn runtime_error(&self, err: &str) -> InterpretResult {
        let line = self.function.chunk.lines[self.ip];
        RuntimeError(line, err.into())
    }
}

pub struct Vm {
    frames: [MaybeUninit<CallFrame>; FRAMES_MAX],
    frame_count: usize,
    objects: Option<Box<Object>>,
    stack: [Value; FRAMES_MAX * STACK_MAX],
    stack_top: usize,
    /// For Globals
    identifiers: HashMap<String, Value>,
}

const FRAME_DEFAULT_VALUE: MaybeUninit<CallFrame> = MaybeUninit::uninit();
const STACK_DEFAULT_VALUE: Value = Value::Uninit;
impl Vm {
    pub fn new() -> Self {
        Self {
            frames: [FRAME_DEFAULT_VALUE; FRAMES_MAX],
            stack: [STACK_DEFAULT_VALUE; FRAMES_MAX * STACK_MAX],
            stack_top: 1,
            frame_count: 0,
            objects: None,
            identifiers: HashMap::new(),
        }
    }

    fn read_operation(&mut self) -> Operation {
        let operation = self.current_chunk().code[self.current_frame().ip];
        self.current_frame_mut().ip += 1;
        operation
    }

    fn runtime_error(&mut self, err: &str) -> InterpretResult {
        let current_frame = self.current_frame_mut();
        current_frame.runtime_error(err)
    }

    fn current_frame(&self) -> &CallFrame {
        unsafe { self.frames[self.frame_count - 1].assume_init_ref() }
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        unsafe { self.frames[self.frame_count - 1].assume_init_mut() }
    }

    fn push_frame(&mut self, frame: CallFrame) {
        self.frames[self.frame_count] = MaybeUninit::new(frame);
        self.frame_count += 1;
    }

    fn current_chunk(&self) -> &Chunk {
        &self.current_frame().function.chunk
    }

    fn push_value(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    fn pop_value(&mut self) -> Value {
        self.stack_top -= 1;
        std::mem::replace(&mut self.stack[self.stack_top], Value::Uninit)
    }

    fn peek_value(&self) -> &Value {
        &self.stack[self.stack_top - 1]
    }

    fn peek_value_mut(&mut self) -> &mut Value {
        &mut self.stack[self.stack_top - 1]
    }

    pub fn run(&mut self) -> InterpretResult {
        if cfg!(feature = "debug_trace_execution") {
            println!("==== Interpreting Chunk ====");
        }
        loop {
            let operation = self.read_operation();
            if cfg!(feature = "debug_trace_execution") {
                self.current_chunk()
                    .disassemble_instruction(self.current_frame().ip - 1);
            }
            match &operation {
                Operation::Return => {
                    return InterpretOk;
                }
                Operation::Constant(index) => {
                    let Some(constant) = self.current_chunk().constants.get(*index as usize) else {
                        return self.runtime_error("No constants initialised.".into());
                    };
                    self.push_value(constant.clone());
                }
                Operation::Negate => {
                    let constant = self.peek_value_mut();
                    if let Err(e) = constant.negate() {
                        return self.runtime_error(&e);
                    };
                }
                Operation::Add => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.add_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => return self.runtime_error(&e),
                    };
                }
                Operation::Subtract => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.sub_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => return self.runtime_error(&e),
                    };
                }
                Operation::Multiply => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.mul_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => return self.runtime_error(&e),
                    };
                }
                Operation::Divide => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.div_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => return self.runtime_error(&e),
                    };
                }
                Operation::Nil => self.push_value(Value::Nil),
                Operation::True => self.push_value(Value::Boolean(true)),
                Operation::False => self.push_value(Value::Boolean(false)),
                Operation::Not => {
                    let constant = self.pop_value();
                    let result = match constant.falsey() {
                        Ok(b) => b,
                        Err(e) => return self.runtime_error(&e),
                    };
                    self.push_value(Value::Boolean(result))
                }
                Operation::Equal => {
                    let (b, a) = (self.pop_value(), self.pop_value());
                    let result = a.is_equal(b);
                    self.push_value(Value::Boolean(result))
                }
                Operation::Greater => {
                    let (b, a) = (self.pop_value(), self.pop_value());
                    match a.is_greater(b) {
                        Ok(result) => self.push_value(Value::Boolean(result)),
                        Err(e) => return self.runtime_error(&e),
                    };
                }
                Operation::Less => {
                    let (b, a) = (self.pop_value(), self.pop_value());
                    match a.is_less(b) {
                        Ok(result) => self.push_value(Value::Boolean(result)),
                        Err(e) => return self.runtime_error(&e),
                    };
                }
                Operation::Print => {
                    let constant = self.pop_value();
                    println!("{constant}");
                }
                Operation::Pop => drop(self.pop_value()),
                Operation::DefineGlobal(index) => {
                    let Some(Value::ObjectValue(Object::String(name))) =
                        self.current_chunk().constants.get(*index as usize)
                    else {
                        return self
                            .runtime_error("No variable initialised with this name.".into());
                    };
                    let name = Rc::clone(&name);
                    let constant = self.pop_value();
                    self.identifiers
                        .insert(name.data.borrow().clone(), constant);
                }
                Operation::GetGlobal(index) => {
                    let Some(Value::ObjectValue(Object::String(name))) =
                        self.current_chunk().constants.get(*index as usize)
                    else {
                        return self
                            .runtime_error("No variable initialised with this name.".into());
                    };
                    let Some(constant) = self.identifiers.get(name.data.borrow().as_str()) else {
                        return self.runtime_error(&format!(
                            "No constant initialised with name '{}'.",
                            name.data.borrow()
                        ));
                    };
                    self.push_value(constant.clone());
                }
                Operation::SetGlobal(index) => {
                    let Some(Value::ObjectValue(Object::String(name))) =
                        self.current_chunk().constants.get(*index as usize)
                    else {
                        return self
                            .runtime_error("No variable initialised with this name.".into());
                    };
                    let name = Rc::clone(&name);
                    let constant = self.peek_value();
                    self.identifiers
                        .insert(name.data.borrow().clone(), constant.clone());
                }
                Operation::GetLocal(slot) => {
                    let index = self.current_frame().start_slot + *slot as usize;
                    let value = &self.stack[index];
                    self.push_value(value.clone());
                }
                Operation::SetLocal(slot) => {
                    let index = self.current_frame().start_slot + *slot as usize;
                    let value_to_replace = self.peek_value();
                    self.stack[index] = value_to_replace.clone();
                }
                Operation::JumpIfFalse(jump) => {
                    let value = self.peek_value();
                    match value.falsey() {
                        Ok(falsy) => {
                            if falsy {
                                self.current_frame_mut().ip += *jump as usize
                            }
                        }
                        Err(e) => {
                            return self.runtime_error(&e);
                        }
                    }
                }
                Operation::Jump(jump) => self.current_frame_mut().ip += *jump as usize,
                Operation::Loop(offset) => self.current_frame_mut().ip -= (*offset + 1) as usize,
            }
        }
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        match compiler::compile(source) {
            Some(function) => {
                self.push_frame(CallFrame {
                    function,
                    ip: 0,
                    start_slot: 0,
                });
            }
            None => return CompileError,
        }
        self.run()
    }
}
