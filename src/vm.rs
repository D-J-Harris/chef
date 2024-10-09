use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::rc::Rc;

use crate::chunk::{Chunk, Operation};
use crate::compiler::Parser;
use crate::objects::{Function, Object};
use crate::value::Value;
use crate::vm::InterpretResult::{CompileError, Ok as InterpretOk};

const FRAMES_MAX: usize = 32;
const STACK_MAX: usize = u8::MAX as usize;

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

struct CallFrame {
    function: Rc<Function>,
    start_slot: usize,
    ip: usize,
}

impl CallFrame {
    fn runtime_error_print(&self) {
        let line = self.function.chunk.lines[self.ip];
        match self.function.name.is_empty() {
            true => eprintln!("[line {line}] in script"),
            false => eprintln!("[line {line}] in {}", self.function.name),
        }
    }
}

pub struct Vm {
    frames: [MaybeUninit<CallFrame>; FRAMES_MAX],
    frame_count: usize,
    _objects: Option<Box<Object>>,
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
            _objects: None,
            identifiers: HashMap::new(),
        }
    }

    fn read_operation(&mut self) -> Operation {
        let operation = self.current_chunk().code[self.current_frame().ip];
        self.current_frame_mut().ip += 1;
        operation
    }

    fn runtime_error(&mut self, err: &str) {
        eprintln!("{}", err);
        for frame_index in (0..self.frame_count).rev() {
            let frame = unsafe { self.frames[frame_index].assume_init_mut() };
            frame.runtime_error_print()
        }
        self.reset_stack();
    }

    fn reset_stack(&mut self) {
        self.stack_top = 0;
        self.frame_count = 0;
    }

    fn current_frame(&self) -> &CallFrame {
        unsafe { self.frames[self.frame_count - 1].assume_init_ref() }
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        unsafe { self.frames[self.frame_count - 1].assume_init_mut() }
    }

    fn push_frame(&mut self, frame: CallFrame) -> Result<(), String> {
        self.frames[self.frame_count] = MaybeUninit::new(frame);
        self.frame_count += 1;
        if self.frame_count == FRAMES_MAX {
            return Err("Stack overflow.".into());
        }
        Ok(())
    }

    fn pop_frame(&mut self) {
        self.frame_count -= 1;
        self.frames[self.frame_count] = MaybeUninit::uninit()
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

    fn peek_value(&self, depth: usize) -> &Value {
        &self.stack[self.stack_top - depth - 1]
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
                    let result = self.pop_value();
                    self.pop_frame();
                    if self.frame_count == 0 {
                        return InterpretOk;
                    }
                    self.push_value(result);
                }
                Operation::Constant(index) => {
                    let Some(constant) = self.current_chunk().constants.get(*index as usize) else {
                        self.runtime_error("No constants initialized.".into());
                        return InterpretResult::RuntimeError;
                    };
                    self.push_value(constant.clone());
                }
                Operation::Negate => {
                    let constant = self.peek_value_mut();
                    if let Err(e) = constant.negate() {
                        self.runtime_error(&e);
                        return InterpretResult::RuntimeError;
                    };
                }
                Operation::Add => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.add_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
                    };
                }
                Operation::Subtract => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.sub_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
                    };
                }
                Operation::Multiply => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.mul_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
                    };
                }
                Operation::Divide => {
                    let (b, mut a) = (self.pop_value(), self.pop_value());
                    match a.div_assign(b) {
                        Ok(()) => self.push_value(a),
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
                    };
                }
                Operation::Nil => self.push_value(Value::Nil),
                Operation::True => self.push_value(Value::Boolean(true)),
                Operation::False => self.push_value(Value::Boolean(false)),
                Operation::Not => {
                    let constant = self.pop_value();
                    let result = match constant.falsey() {
                        Ok(b) => b,
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
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
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
                    };
                }
                Operation::Less => {
                    let (b, a) = (self.pop_value(), self.pop_value());
                    match a.is_less(b) {
                        Ok(result) => self.push_value(Value::Boolean(result)),
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
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
                        self.runtime_error("No variable initialized with this name.".into());
                        return InterpretResult::RuntimeError;
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
                        self.runtime_error("No variable initialized with this name.".into());
                        return InterpretResult::RuntimeError;
                    };
                    let Some(constant) = self.identifiers.get(name.data.borrow().as_str()) else {
                        self.runtime_error(&format!(
                            "No constant initialized with name '{}'.",
                            name.data.borrow()
                        ));
                        return InterpretResult::RuntimeError;
                    };
                    self.push_value(constant.clone());
                }
                Operation::SetGlobal(index) => {
                    let Some(Value::ObjectValue(Object::String(name))) =
                        self.current_chunk().constants.get(*index as usize)
                    else {
                        self.runtime_error("No variable initialized with this name.".into());
                        return InterpretResult::RuntimeError;
                    };
                    let name = Rc::clone(&name);
                    let constant = self.peek_value(0);
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
                    let value_to_replace = self.peek_value(0);
                    self.stack[index] = value_to_replace.clone();
                }
                Operation::JumpIfFalse(jump) => {
                    let value = self.peek_value(0);
                    match value.falsey() {
                        Ok(falsy) => {
                            if falsy {
                                self.current_frame_mut().ip += *jump as usize
                            }
                        }
                        Err(e) => {
                            self.runtime_error(&e);
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                Operation::Jump(jump) => self.current_frame_mut().ip += *jump as usize,
                Operation::Loop(offset) => self.current_frame_mut().ip -= (*offset + 1) as usize,
                Operation::Call(argument_count) => {
                    if let Err(e) = self.call_value(*argument_count) {
                        self.runtime_error(&e);
                        return InterpretResult::RuntimeError;
                    }
                }
            }
        }
    }

    fn call_value(&mut self, argument_count: u8) -> Result<(), String> {
        let callee = self.peek_value(argument_count as usize);
        match callee {
            Value::Uninit
            | Value::Nil
            | Value::Number(_)
            | Value::Boolean(_)
            | Value::ObjectValue(Object::String(_)) => {
                return Err("Can only call functions and classes.".into());
            }
            Value::ObjectValue(Object::Function(function)) => {
                self.call(Rc::clone(&function), argument_count)?;
                Ok(())
            }
        }
    }

    fn call(&mut self, function: Rc<Function>, argument_count: u8) -> Result<(), String> {
        if function.arity != argument_count {
            return Err(format!(
                "Expected {} arguments but got {argument_count}.",
                function.arity
            ));
        }
        self.push_frame(CallFrame {
            function,
            start_slot: self.stack_top - 1 - argument_count as usize,
            ip: 0,
        })?;
        Ok(())
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let parser = Parser::new(source);
        match parser.compile() {
            Some(function) => self
                .call(function, 0)
                .expect("Failed to call top-level script."),
            None => return CompileError,
        }
        self.run()
    }
}
