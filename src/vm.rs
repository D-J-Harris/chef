use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use crate::chunk::Operation;
use crate::common::U8_MAX;
use crate::compiler::Parser;
use crate::objects::{ClosureObject, FunctionObject, UpvalueObject};
use crate::value::Value;
use crate::vm::InterpretResult::{CompileError, Ok as InterpretOk};

const FRAMES_MAX: usize = 64;
const STACK_MAX: usize = U8_MAX;

#[derive(Debug)]
pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

struct CallFrame {
    closure: Rc<ClosureObject>,
    slot: usize,
    ip: usize,
}

impl CallFrame {
    fn runtime_error_print(&self) {
        let function = self
            .closure
            .function
            .upgrade()
            .expect("No current function to inspect for runtime error.");
        let line = function.chunk.lines[self.ip];
        match function.name.is_empty() {
            true => eprintln!("[line {line}] in script"),
            false => eprintln!("[line {line}] in {}", function.name),
        }
    }
}

pub struct Vm {
    frames: [Option<CallFrame>; FRAMES_MAX],
    frame_count: usize,
    stack: [Option<Value>; FRAMES_MAX * STACK_MAX],
    stack_top: usize,
    open_upvalues: Vec<Rc<RefCell<UpvalueObject>>>,
    /// For Globals
    pub identifiers: HashMap<String, Value>,
}

const FRAME_DEFAULT_VALUE: Option<CallFrame> = None;
const STACK_DEFAULT_VALUE: Option<Value> = None;
impl Vm {
    pub fn new() -> Self {
        let mut vm = Self {
            frames: [FRAME_DEFAULT_VALUE; FRAMES_MAX],
            stack: [STACK_DEFAULT_VALUE; FRAMES_MAX * STACK_MAX],
            stack_top: 0,
            frame_count: 0,
            open_upvalues: Vec::new(),
            identifiers: HashMap::new(),
        };
        vm.declare_native_functions();
        vm
    }

    fn reset(&mut self) {
        self.stack_top = 0;
        drop(std::mem::replace(
            &mut self.stack,
            [STACK_DEFAULT_VALUE; FRAMES_MAX * STACK_MAX],
        ));
        self.frame_count = 0;
        drop(std::mem::replace(
            &mut self.frames,
            [FRAME_DEFAULT_VALUE; FRAMES_MAX],
        ));
    }

    fn read_operation(&mut self) -> Operation {
        let operation = self.current_function().chunk.code[self.current_frame().ip];
        self.current_frame_mut().ip += 1;
        operation
    }

    fn runtime_error(&mut self, err: &str) {
        eprintln!("{}", err);
        for frame_index in (0..self.frame_count).rev() {
            let frame = self.frames[frame_index]
                .as_ref()
                .expect("Could not find stack frame.");
            frame.runtime_error_print()
        }
        self.reset();
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames[self.frame_count - 1]
            .as_ref()
            .expect("Could not find stack frame.")
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames[self.frame_count - 1]
            .as_mut()
            .expect("Could not find stack frame.")
    }

    fn push_frame(&mut self, frame: CallFrame) -> Result<(), String> {
        self.frames[self.frame_count] = Some(frame);
        self.frame_count += 1;
        if self.frame_count == FRAMES_MAX {
            return Err("Stack overflow.".into());
        }
        Ok(())
    }

    fn pop_frame(&mut self) -> CallFrame {
        self.frame_count -= 1;
        let frame = std::mem::replace(&mut self.frames[self.frame_count], None);
        frame.expect("No frame to pop from stack.")
    }

    fn current_function(&self) -> Rc<FunctionObject> {
        self.current_frame()
            .closure
            .function
            .upgrade()
            .expect("Function reference has been dropped")
    }

    fn push_value(&mut self, value: Value) {
        self.stack[self.stack_top] = Some(value);
        self.stack_top += 1;
    }

    fn pop_value(&mut self) -> Value {
        self.stack_top -= 1;
        let value = std::mem::replace(&mut self.stack[self.stack_top], None);
        value.expect("No value to pop from stack.")
    }

    fn peek_value(&self, depth: usize) -> Option<&Value> {
        self.stack
            .get(self.stack_top.checked_sub(depth + 1)?)
            .and_then(|opt| opt.as_ref())
    }

    pub fn run(&mut self) -> InterpretResult {
        #[cfg(feature = "debug_trace_execution")]
        println!("==== Interpreting Chunk ====");
        loop {
            let operation = self.read_operation();
            #[cfg(feature = "debug_trace_execution")]
            self.current_function()
                .chunk
                .disassemble_instruction(self.current_frame().ip - 1);
            match &operation {
                Operation::Return => {
                    let result = self.pop_value();
                    let frame = self.pop_frame();
                    if self.frame_count == 0 {
                        self.pop_value();
                        return InterpretOk;
                    }
                    self.stack_top = frame.slot;
                    self.push_value(result);
                }
                Operation::Constant(index) => {
                    let constant =
                        match self.current_function().chunk.constants.get(*index as usize) {
                            Some(Some(constant)) => constant.clone(),
                            _ => {
                                self.runtime_error("No constants initialized.".into());
                                return InterpretResult::RuntimeError;
                            }
                        };
                    self.push_value(constant);
                }
                Operation::Negate => {
                    let mut constant = self.pop_value();
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
                    let name = match self.current_function().chunk.constants.get(*index as usize) {
                        Some(Some(Value::String(name))) => name.clone(),
                        _ => {
                            self.runtime_error("No global varibale name initialized.".into());
                            return InterpretResult::RuntimeError;
                        }
                    };
                    let constant = self.pop_value();
                    self.identifiers.insert(name, constant);
                }
                Operation::GetGlobal(index) => {
                    // TODO: unnecessary cloning and then referencing
                    let name = match self.current_function().chunk.constants.get(*index as usize) {
                        Some(Some(Value::String(name))) => name.clone(),
                        _ => {
                            self.runtime_error("No global variable initialized.".into());
                            return InterpretResult::RuntimeError;
                        }
                    };
                    let Some(constant) = self.identifiers.get(&name) else {
                        self.runtime_error(&format!(
                            "No constant initialized with name '{}'.",
                            name
                        ));
                        return InterpretResult::RuntimeError;
                    };
                    self.push_value(constant.clone());
                }
                Operation::SetGlobal(index) => {
                    let name = match self.current_function().chunk.constants.get(*index as usize) {
                        Some(Some(Value::String(name))) => name.clone(),
                        _ => {
                            self.runtime_error("No global variable initialized.".into());
                            return InterpretResult::RuntimeError;
                        }
                    };
                    let Some(constant) = self.peek_value(0) else {
                        self.runtime_error("No value on top of stack to set with.".into());
                        return InterpretResult::RuntimeError;
                    };
                    self.identifiers.insert(name.clone(), constant.clone());
                }
                Operation::GetLocal(slot) => {
                    let index = self.current_frame().slot + *slot as usize;
                    let Some(value) = &self.stack[index] else {
                        self.runtime_error("No local variable initialized with this name.".into());
                        return InterpretResult::RuntimeError;
                    };
                    self.push_value(value.clone());
                }
                Operation::SetLocal(slot) => {
                    let index = self.current_frame().slot + *slot as usize;
                    let Some(value_to_replace) = self.peek_value(0) else {
                        self.runtime_error("No value on top of stack to set with.".into());
                        return InterpretResult::RuntimeError;
                    };
                    self.stack[index] = Some(value_to_replace.clone());
                }
                Operation::JumpIfFalse(jump) => {
                    let Some(value) = self.peek_value(0) else {
                        self.runtime_error("No value on top of stack to jump with.".into());
                        return InterpretResult::RuntimeError;
                    };
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
                Operation::Closure(index) => {
                    let (function_name, function_upvalue_count, function) =
                        match self.current_function().chunk.constants.get(*index as usize) {
                            Some(Some(Value::Function(function))) => (
                                function.name.clone(),
                                function.upvalue_count,
                                Rc::downgrade(&function),
                            ),
                            _ => {
                                self.runtime_error(
                                    "No function initialized for this closure.".into(),
                                );
                                return InterpretResult::RuntimeError;
                            }
                        };
                    let mut closure_object =
                        ClosureObject::new(&function_name, function_upvalue_count, function);
                    for i in 0..closure_object.upvalue_count {
                        let Operation::ClosureIsLocalByte(is_local) = self.read_operation() else {
                            self.runtime_error("Expected closure is_local byte");
                            return InterpretResult::RuntimeError;
                        };
                        let Operation::ClosureIndexByte(index) = self.read_operation() else {
                            self.runtime_error("Expected closure index byte");
                            return InterpretResult::RuntimeError;
                        };
                        if is_local {
                            closure_object.upvalues[i as usize] = Some(
                                self.capture_upvalue(self.current_frame().slot + index as usize),
                            )
                        } else {
                            let Some(upvalue) = &self.current_frame().closure.upvalues[i as usize]
                            else {
                                self.runtime_error("Invalid upvalue location");
                                return InterpretResult::RuntimeError;
                            };
                            closure_object.upvalues[i as usize] = Some(upvalue.clone())
                        }
                    }
                    self.push_value(Value::Closure(Rc::new(closure_object)));
                }
                Operation::GetUpvalue(upvalue_slot) => {
                    let slot = *upvalue_slot as usize;
                    let Some(upvalue) = &self.current_frame().closure.upvalues[slot] else {
                        self.runtime_error("No upvalue to get.");
                        return InterpretResult::RuntimeError;
                    };
                    let value = match &*upvalue.borrow() {
                        UpvalueObject::Open(value_slot) => {
                            let Some(value) = &self.stack[*value_slot] else {
                                panic!("No value at open upvalue location."); // TODO: propagate properly
                            };
                            value.clone()
                        }
                        UpvalueObject::Closed(value) => value.clone(),
                    };
                    self.push_value(value);
                }
                Operation::SetUpvalue(upvalue_slot) => {
                    let slot = *upvalue_slot as usize;
                    let Some(replacement_value) = self.peek_value(0) else {
                        self.runtime_error("No values on the stack.");
                        return InterpretResult::RuntimeError;
                    };
                    let replacement_value = replacement_value.clone();
                    let Some(ref upvalue) = self.current_frame().closure.upvalues[slot] else {
                        self.runtime_error("No upvalue to get.");
                        return InterpretResult::RuntimeError;
                    };
                    let slot = *match &mut *upvalue.borrow_mut() {
                        UpvalueObject::Open(value_slot) => value_slot,
                        UpvalueObject::Closed(value) => {
                            *value = replacement_value;
                            continue;
                        }
                    };
                    self.stack[slot] = Some(replacement_value)
                }
                Operation::CloseUpvalue => {
                    self.close_upvalues(self.stack_top - 1);
                    self.pop_value();
                }
                Operation::ClosureIsLocalByte(_) => unreachable!(),
                Operation::ClosureIndexByte(_) => unreachable!(),
            }
        }
    }

    fn close_upvalues(&mut self, from: usize) {
        for upvalue in self.open_upvalues.iter().rev() {
            let slot = match *upvalue.borrow() {
                UpvalueObject::Open(value_slot) => match value_slot < from {
                    true => continue,
                    false => value_slot,
                },
                UpvalueObject::Closed(_) => continue,
            };
            let Some(ref value) = self.stack[slot] else {
                self.runtime_error("No value to close over."); // TODO: propagate better
                return;
            };
            upvalue.replace(UpvalueObject::Closed(value.clone()));
        }
        self.open_upvalues
            .retain(|upvalue| match *upvalue.borrow() {
                UpvalueObject::Open(_) => true,
                UpvalueObject::Closed(_) => false,
            });
    }

    fn capture_upvalue(&mut self, value_slot: usize) -> Rc<RefCell<UpvalueObject>> {
        for upvalue in self.open_upvalues.iter().rev() {
            match *upvalue.borrow() {
                UpvalueObject::Open(slot) => {
                    if slot == value_slot {
                        return Rc::clone(&upvalue);
                    }
                }
                UpvalueObject::Closed(_) => continue,
            }
        }
        let upvalue = Rc::new(RefCell::new(UpvalueObject::new(value_slot)));
        self.open_upvalues.push(Rc::clone(&upvalue));
        upvalue
    }

    fn call_value(&mut self, argument_count: u8) -> Result<(), String> {
        let Some(callee) = self.peek_value(argument_count as usize) else {
            return Err("No callee in the stack.".into());
        };
        match callee {
            Value::NativeFunction(function) => {
                let result =
                    (function.function)(argument_count, self.stack_top - argument_count as usize);
                self.stack_top -= argument_count as usize + 1;
                self.push_value(result);
                Ok(())
            }
            Value::Closure(closure) => {
                self.call(Rc::clone(&closure), argument_count)?;
                Ok(())
            }
            _ => Err("Can only call functions and classes.".into()),
        }
    }

    fn call(&mut self, closure: Rc<ClosureObject>, argument_count: u8) -> Result<(), String> {
        let Some(function) = closure.function.upgrade() else {
            return Err("Call to closure does not have an associated function".into());
        };
        if function.arity != argument_count {
            return Err(format!(
                "Expected {} arguments but got {argument_count}.",
                function.arity
            ));
        }
        self.push_frame(CallFrame {
            slot: self.stack_top - (argument_count as usize + 1),
            closure,
            ip: 0,
        })?;
        Ok(())
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult {
        let parser = Parser::new(source);
        match parser.compile() {
            Some(function) => {
                // insert script as global
                let function = Rc::new(function);
                self.push_value(Value::Function(Rc::clone(&function)));
                let closure = Rc::new(ClosureObject::new(
                    &function.name,
                    function.upvalue_count,
                    Rc::downgrade(&function),
                ));
                self.push_value(Value::Closure(Rc::clone(&closure)));
                self.call(closure, 0)
                    .expect("Failed to call top-level script.")
            }
            None => return CompileError,
        }
        let result = self.run();
        println!("\nProgramme Finished\n");
        result
    }
}
