use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::chunk::Operation;
use crate::common::{FRAMES_MAX_COUNT, INIT_STRING, STACK_VALUES_MAX_COUNT};
use crate::compiler::Parser;
use crate::error::{InterpretResult, RuntimeError};
use crate::objects::{
    BoundMethod, ClassObject, ClosureObject, FunctionObject, InstanceObject, UpvalueObject,
};
use crate::value::Value;

#[derive(Debug)]
struct CallFrame {
    closure: Rc<ClosureObject>,
    slot: usize,
    ip: usize,
}

impl CallFrame {
    fn runtime_error_print(&self) {
        let function = &self.closure.function;
        let line = function.chunk.lines[self.ip - 1];
        match function.name.is_empty() {
            true => eprintln!("[line {line}] in script"),
            false => eprintln!("[line {line}] in {}", function.name),
        }
    }
}

pub struct Vm {
    frames: [Option<CallFrame>; FRAMES_MAX_COUNT],
    frame_count: usize,
    stack: [Option<Value>; STACK_VALUES_MAX_COUNT],
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
            frames: [FRAME_DEFAULT_VALUE; FRAMES_MAX_COUNT],
            stack: [STACK_DEFAULT_VALUE; STACK_VALUES_MAX_COUNT],
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
            [STACK_DEFAULT_VALUE; STACK_VALUES_MAX_COUNT],
        ));
        self.frame_count = 0;
        drop(std::mem::replace(
            &mut self.frames,
            [FRAME_DEFAULT_VALUE; FRAMES_MAX_COUNT],
        ));
        self.open_upvalues.truncate(0);
    }

    fn read_operation(&mut self) -> InterpretResult<Operation> {
        let operation = self.current_function().chunk.code[self.current_frame().ip];
        self.current_frame_mut().ip += 1;
        Ok(operation)
    }

    pub fn stack_error(&mut self) {
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

    fn push_frame(&mut self, frame: CallFrame) -> InterpretResult<()> {
        if self.frame_count + 1 == FRAMES_MAX_COUNT {
            return Err(RuntimeError::StackOverflow);
        }
        self.frames[self.frame_count] = Some(frame);
        self.frame_count += 1;
        Ok(())
    }

    fn pop_frame(&mut self) -> CallFrame {
        self.frame_count -= 1;
        let frame = std::mem::replace(&mut self.frames[self.frame_count], None);
        frame.expect("No frame to pop from stack.")
    }

    fn current_function(&self) -> Rc<FunctionObject> {
        Rc::clone(&self.current_frame().closure.function)
    }

    fn read_constant(&self, index: u8) -> InterpretResult<Value> {
        Ok(self
            .current_function()
            .chunk
            .constants
            .get(index as usize)
            .ok_or(RuntimeError::OutOfBounds)?
            .as_ref()
            .ok_or(RuntimeError::UninitializedConstantValue)?
            .clone())
    }

    fn push(&mut self, value: Value) -> InterpretResult<()> {
        if self.stack_top == STACK_VALUES_MAX_COUNT {
            return Err(RuntimeError::StackOverflow);
        }
        self.stack[self.stack_top] = Some(value);
        self.stack_top += 1;
        Ok(())
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        let value = std::mem::replace(&mut self.stack[self.stack_top], None);
        value.expect("No value to pop from stack.")
    }

    fn current_slot(&self) -> usize {
        self.stack_top - 1
    }

    fn peek(&self, slot: usize) -> InterpretResult<&Value> {
        Ok(self
            .stack
            .get(slot)
            .ok_or(RuntimeError::OutOfBounds)?
            .as_ref()
            .ok_or(RuntimeError::UninitializedStackValue)?)
    }

    fn peek_mut(&mut self, slot: usize) -> InterpretResult<&mut Value> {
        Ok(self
            .stack
            .get_mut(slot)
            .ok_or(RuntimeError::OutOfBounds)?
            .as_mut()
            .ok_or(RuntimeError::UninitializedStackValue)?)
    }

    pub fn run(&mut self) -> InterpretResult<()> {
        #[cfg(feature = "debug_trace_execution")]
        println!("==== Interpreting Chunk ====");
        loop {
            let operation = self.read_operation()?;
            #[cfg(feature = "debug_trace_execution")]
            self.current_function()
                .chunk
                .disassemble_instruction(self.current_frame().ip - 1);
            match &operation {
                Operation::Return => {
                    let frame = self.pop_frame();
                    let result = self.pop();
                    self.close_upvalues(frame.slot)?;
                    if self.frame_count == 0 {
                        return Ok(());
                    }
                    // Unwind the current call frame from the stack.
                    loop {
                        drop(self.pop());
                        if self.stack_top == frame.slot {
                            break;
                        }
                    }
                    self.push(result)?
                }
                Operation::Constant(index) => {
                    let value = self.read_constant(*index)?;
                    self.push(value)?
                }
                Operation::Negate => {
                    let constant = self.peek_mut(self.current_slot())?;
                    constant.negate()?
                }
                Operation::Add => {
                    let (b, mut a) = (self.pop(), self.pop());
                    a.add_assign(b)?;
                    self.push(a)?
                }
                Operation::Subtract => {
                    let (b, mut a) = (self.pop(), self.pop());
                    a.sub_assign(b)?;
                    self.push(a)?
                }
                Operation::Multiply => {
                    let (b, mut a) = (self.pop(), self.pop());
                    a.mul_assign(b)?;
                    self.push(a)?
                }
                Operation::Divide => {
                    let (b, mut a) = (self.pop(), self.pop());
                    a.div_assign(b)?;
                    self.push(a)?
                }
                Operation::Nil => self.push(Value::Nil)?,
                Operation::True => self.push(Value::Boolean(true))?,
                Operation::False => self.push(Value::Boolean(false))?,
                Operation::Not => {
                    let constant = self.pop();
                    let result = constant.falsey();
                    self.push(Value::Boolean(result))?
                }
                Operation::Equal => {
                    let (b, a) = (self.pop(), self.pop());
                    let result = a.is_equal(b);
                    self.push(Value::Boolean(result))?
                }
                Operation::Greater => {
                    let (b, a) = (self.pop(), self.pop());
                    let result = a.is_greater(b)?;
                    self.push(Value::Boolean(result))?
                }
                Operation::Less => {
                    let (b, a) = (self.pop(), self.pop());
                    let result = a.is_less(b)?;
                    self.push(Value::Boolean(result))?
                }
                Operation::Print => {
                    let constant = self.pop();
                    println!("{constant}");
                }
                Operation::Pop => drop(self.pop()),
                Operation::DefineGlobal(index) => {
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let constant = self.pop();
                    self.identifiers.insert(name, constant);
                }
                Operation::GetGlobal(index) => {
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let constant = self
                        .identifiers
                        .get(&name)
                        .ok_or(RuntimeError::UndefinedVariable(name))?;
                    self.push(constant.clone())?
                }
                Operation::SetGlobal(index) => {
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let constant = self.peek(self.current_slot())?;
                    self.identifiers
                        .insert(name.clone(), constant.clone())
                        .ok_or(RuntimeError::UndefinedVariable(name))?;
                }
                Operation::GetLocal(frame_slot) => {
                    let slot = self.current_frame().slot + *frame_slot as usize;
                    let value = self.peek(slot)?;
                    self.push(value.clone())?
                }
                Operation::SetLocal(frame_slot) => {
                    let slot = self.current_frame().slot + *frame_slot as usize;
                    let replacement_value = self.peek(self.current_slot())?;
                    *self.peek_mut(slot)? = replacement_value.clone();
                }
                Operation::JumpIfFalse(jump) => {
                    let value = self.peek(self.current_slot())?;
                    if value.falsey() {
                        self.current_frame_mut().ip += *jump as usize;
                    }
                }
                Operation::Jump(jump) => self.current_frame_mut().ip += *jump as usize,
                Operation::Loop(offset) => self.current_frame_mut().ip -= (*offset + 1) as usize,
                Operation::Call(argument_count) => self.call_value(*argument_count)?,
                Operation::Closure(index) => {
                    let Value::Function(function) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantFunctionNotFound);
                    };
                    let (upvalue_count, function) = (function.upvalue_count, Rc::clone(&function));
                    let mut closure_object = ClosureObject::new(upvalue_count, function);
                    // TODO: investigate adding upvalues "index" and "is_local" to array in function, no need to emit bytes
                    for i in 0..upvalue_count {
                        let Operation::ClosureIsLocalByte(is_local) = self.read_operation()? else {
                            return Err(RuntimeError::ClosureOpcode);
                        };
                        let Operation::ClosureIndexByte(index) = self.read_operation()? else {
                            return Err(RuntimeError::ClosureOpcode);
                        };
                        let upvalue = if is_local {
                            let upvalue_slot = self.current_frame().slot + index as usize;
                            self.capture_upvalue(upvalue_slot)
                        } else {
                            Rc::clone(
                                &self.current_frame().closure.upvalues[index as usize]
                                    .as_ref()
                                    .unwrap(),
                            )
                        };
                        closure_object.upvalues[i as usize].replace(upvalue);
                    }
                    self.push(Value::Closure(Rc::new(closure_object)))?
                }
                Operation::GetUpvalue(upvalue_slot) => {
                    let slot = *upvalue_slot as usize;
                    let Some(upvalue) = &self.current_frame().closure.upvalues[slot] else {
                        return Err(RuntimeError::OutOfBounds);
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
                    self.push(value)?
                }
                Operation::SetUpvalue(upvalue_slot) => {
                    let slot = *upvalue_slot as usize;
                    let replacement_value = self.peek(self.current_slot())?.clone();
                    let Some(ref upvalue) = self.current_frame().closure.upvalues[slot] else {
                        return Err(RuntimeError::OutOfBounds);
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
                    self.close_upvalues(self.current_slot())?;
                    self.pop();
                }
                Operation::ClosureIsLocalByte(_) => unreachable!(),
                Operation::ClosureIndexByte(_) => unreachable!(),
                Operation::Class(index) => {
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantClassNotFound);
                    };
                    let class = Rc::new(RefCell::new(ClassObject::new(&name)));
                    self.push(Value::Class(class))?
                }
                Operation::GetProperty(index) => {
                    let Value::Instance(instance) = self.peek(self.current_slot())? else {
                        return Err(RuntimeError::InstanceGetProperty);
                    };
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let instance = Rc::clone(instance);
                    match instance.borrow().fields.get(&name) {
                        Some(value) => {
                            let value = value.try_into()?;
                            self.pop();
                            self.push(value)?;
                            continue;
                        }
                        None => {
                            let instance = Rc::clone(&instance);
                            let class = &instance.borrow().class;
                            self.bind_method(name, class)?;
                        }
                    };
                }
                Operation::SetProperty(index) => {
                    let Value::Instance(instance) = self.peek(self.current_slot() - 1)? else {
                        return Err(RuntimeError::InstanceSetProperty);
                    };
                    let instance = Rc::clone(instance);
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let value = self.pop();
                    instance
                        .borrow_mut()
                        .fields
                        .insert(name, value.clone().into());
                    self.pop();
                    self.push(value)?
                }
                Operation::Method(index) => {
                    let Value::String(name) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    self.define_method(name)?;
                }
                Operation::Invoke(index, argument_count) => {
                    let Value::String(method) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    self.invoke(&method, *argument_count)?;
                }
                Operation::Inherit => {
                    let Value::Class(superclass) = self.peek(self.current_slot() - 1)? else {
                        return Err(RuntimeError::ConstantSuperclassNotFound);
                    };
                    let Value::Class(subclass) = self.peek(self.current_slot())? else {
                        return Err(RuntimeError::ConstantClassNotFound);
                    };
                    for (name, method) in superclass.borrow().methods.iter() {
                        subclass
                            .borrow_mut()
                            .methods
                            .insert(name.clone(), Rc::clone(&method));
                    }
                    self.pop();
                }
                Operation::GetSuper(index) => {
                    let Value::String(method) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let Value::Class(superclass) = self.pop() else {
                        return Err(RuntimeError::ConstantSuperclassNotFound);
                    };
                    self.bind_method(method, superclass.as_ref())?;
                }
                Operation::SuperInvoke(index, argument_count) => {
                    let Value::String(method) = self.read_constant(*index)? else {
                        return Err(RuntimeError::ConstantStringNotFound);
                    };
                    let Value::Class(superclass) = self.pop() else {
                        return Err(RuntimeError::ConstantSuperclassNotFound);
                    };
                    self.invoke_from_class(&superclass, &method, *argument_count)?;
                }
            }
        }
    }

    fn invoke(&mut self, method: &str, argument_count: u8) -> InterpretResult<()> {
        let Value::Instance(receiver) = self.peek(self.current_slot() - argument_count as usize)?
        else {
            return Err(RuntimeError::InstanceInvoke);
        };
        if let Some(field_value) = Rc::clone(&receiver).borrow().fields.get(method) {
            let value = field_value.try_into()?;
            self.stack[self.current_slot() - argument_count as usize] = Some(value);
            return self.call_value(argument_count);
        }
        self.invoke_from_class(&Rc::clone(&receiver).borrow().class, method, argument_count)
    }

    fn invoke_from_class(
        &mut self,
        class: &Rc<RefCell<ClassObject>>,
        name: &str,
        argument_count: u8,
    ) -> InterpretResult<()> {
        let class_borrow = class.borrow();
        let method = class_borrow
            .methods
            .get(name)
            .ok_or(RuntimeError::UndefinedProperty(name.into()))?;
        self.call(Rc::clone(&method), argument_count)
    }

    fn define_method(&mut self, name: String) -> InterpretResult<()> {
        let Value::Closure(closure) = self.pop() else {
            return Err(RuntimeError::ConstantClosureNotFound);
        };
        let Value::Class(class) = self.peek(self.current_slot())? else {
            return Err(RuntimeError::ConstantClassNotFound);
        };
        class.borrow_mut().add_method(name, closure);
        Ok(())
    }

    fn close_upvalues(&mut self, from: usize) -> InterpretResult<()> {
        for upvalue in self.open_upvalues.iter() {
            let slot = match *upvalue.borrow() {
                UpvalueObject::Open(value_slot) => match value_slot < from {
                    true => continue,
                    false => value_slot,
                },
                UpvalueObject::Closed(_) => continue,
            };
            let value = self
                .stack
                .get(slot)
                .ok_or(RuntimeError::OutOfBounds)?
                .as_ref()
                .ok_or(RuntimeError::UninitializedStackValue)?;
            upvalue.replace(UpvalueObject::Closed(value.clone()));
        }
        self.open_upvalues
            .retain(|upvalue| match *upvalue.borrow() {
                UpvalueObject::Open(_) => true,
                UpvalueObject::Closed(_) => false,
            });
        Ok(())
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

    fn call_value(&mut self, argument_count: u8) -> InterpretResult<()> {
        let callee = self
            .peek(self.current_slot() - argument_count as usize)?
            .clone(); // TODO: can mutable behaviour be allowed without cloning the peek here?
        match callee {
            Value::NativeFunction(function) => {
                let result =
                    (function.function)(argument_count, self.stack_top - argument_count as usize);
                self.stack_top -= argument_count as usize + 1;
                self.push(result)?;
                Ok(())
            }
            Value::Closure(closure) => {
                self.call(Rc::clone(&closure), argument_count)?;
                Ok(())
            }
            Value::Class(class) => {
                let instance = Value::Instance(Rc::new(RefCell::new(InstanceObject::new(
                    Rc::clone(&class),
                ))));
                self.stack[self.current_slot() - argument_count as usize].replace(instance);
                if let Some(closure) = class.borrow().methods.get(INIT_STRING) {
                    self.call(Rc::clone(&closure), argument_count)?;
                } else if argument_count != 0 {
                    return Err(RuntimeError::ClassArguments(argument_count));
                }
                Ok(())
            }
            Value::BoundMethod(bound_method) => {
                let closure = bound_method.closure;
                self.stack[self.current_slot() - argument_count as usize] =
                    Some(Value::Instance(Rc::clone(&bound_method.receiver)));
                self.call(closure, argument_count)?;
                Ok(())
            }
            _ => Err(RuntimeError::InvalidCallee),
        }
    }

    fn call(&mut self, closure: Rc<ClosureObject>, argument_count: u8) -> InterpretResult<()> {
        let function = &closure.function;
        if function.arity != argument_count {
            return Err(RuntimeError::FunctionArity(function.arity, argument_count));
        }
        self.push_frame(CallFrame {
            slot: self.stack_top - (argument_count as usize + 1),
            closure,
            ip: 0,
        })?;
        Ok(())
    }

    fn bind_method(&mut self, name: String, class: &RefCell<ClassObject>) -> InterpretResult<()> {
        let class = class.borrow();
        let closure = class
            .methods
            .get(&name)
            .ok_or(RuntimeError::UndefinedProperty(name))?;
        let Value::Instance(receiver) = self.pop() else {
            panic!("instance not on stack"); // TODO: cleanup
        };
        let bound_method = BoundMethod::new(Rc::clone(&receiver), Rc::clone(&closure));
        self.push(Value::BoundMethod(bound_method))
    }

    pub fn interpret(&mut self, source: &str) -> InterpretResult<()> {
        let parser = Parser::new(source);
        let Some(function) = parser.compile() else {
            return Err(RuntimeError::CompileError); // TODO: can propagate this error from parser.compile()
        };
        let function = Rc::new(function);
        self.push(Value::Function(Rc::clone(&function)))?;
        let closure = Rc::new(ClosureObject::new(
            function.upvalue_count,
            Rc::clone(&function),
        ));
        self.call(closure, 0)
            .expect("Failed to call top-level script.");
        let result = self.run();
        if let Err(err) = &result {
            match err {
                RuntimeError::CompileError => (),
                _ => {
                    eprintln!("{err}");
                    self.stack_error();
                }
            }
        };
        result
    }
}
