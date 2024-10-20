use std::collections::HashMap;
use std::ops::Deref;

use gc_arena::lock::RefLock;
use gc_arena::{Collect, Collection, Gc, Mutation};

use crate::chunk::Operation;
use crate::common::{CALL_FRAMES_MAX_COUNT, INIT_STRING, STACK_VALUES_MAX_COUNT};
use crate::error::{ChefError, InterpretResult};
use crate::native_functions::current_time_s;
use crate::objects::{
    BoundMethod, ClassObject, ClosureObject, FunctionObject, InstanceObject, UpvalueObject,
};
use crate::value::Value;

#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct CallFrame<'gc> {
    closure: Gc<'gc, ClosureObject<'gc>>,
    stack_index: usize,
    frame_ip: usize,
}

impl<'gc> CallFrame<'gc> {
    fn runtime_error_print(&self) {
        let function = &self.closure.function;
        let line = function.chunk.lines[self.frame_ip - 1];
        match function.name.is_empty() {
            true => eprintln!("[line {line}] in script"),
            false => eprintln!("[line {line}] in {}", function.name),
        }
    }

    fn closure(&self) -> &ClosureObject<'gc> {
        self.closure.deref()
    }

    fn function(&self) -> &FunctionObject<'gc> {
        self.closure().function.deref()
    }
}

pub struct State<'gc> {
    mc: &'gc Mutation<'gc>,
    frames: Vec<CallFrame<'gc>>,
    stack: Vec<Value<'gc>>,
    stack_top: usize,
    upvalues: Vec<Gc<'gc, RefLock<UpvalueObject<'gc>>>>,
    pub(super) identifiers: HashMap<String, Value<'gc>>,
}

unsafe impl<'gc> Collect for State<'gc> {
    fn needs_trace() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn trace(&self, cc: &Collection) {
        self.frames.trace(cc);
        self.stack.trace(cc);
        self.stack_top.trace(cc);
        self.upvalues.trace(cc);
        self.identifiers.trace(cc);
    }
}

impl<'gc> State<'gc> {
    pub fn new(mc: &'gc Mutation<'gc>) -> Self {
        let identifiers = HashMap::new();

        Self {
            mc,
            frames: Vec::with_capacity(CALL_FRAMES_MAX_COUNT),
            stack: Vec::with_capacity(STACK_VALUES_MAX_COUNT),
            stack_top: 0,
            upvalues: Vec::new(),
            identifiers,
        }
    }

    pub fn declare_native_functions(&mut self) {
        let name = "clock".into();
        let native_function = current_time_s::<'gc>;
        let function = Value::NativeFunction(native_function);
        self.identifiers.insert(name, function);
    }

    fn reset(&mut self) {
        self.stack.truncate(0);
        self.frames.truncate(0);
        self.upvalues.truncate(0);
    }

    pub fn stack_error(&mut self) {
        for frame in self.frames.iter().rev() {
            frame.runtime_error_print()
        }
        self.reset();
    }

    pub(super) fn push_frame(&mut self, frame: CallFrame<'gc>) -> InterpretResult<()> {
        if self.frames.len() == CALL_FRAMES_MAX_COUNT {
            return Err(ChefError::StackOverflow);
        }
        self.frames.push(frame);
        Ok(())
    }

    fn pop_frame(&mut self) -> CallFrame<'gc> {
        let frame = self.frames.pop();
        frame.expect("No frame to pop from stack.")
    }

    pub(super) fn push(&mut self, value: Value<'gc>) -> InterpretResult<()> {
        if self.stack.len() == STACK_VALUES_MAX_COUNT {
            return Err(ChefError::StackOverflow);
        }
        self.stack.push(value);
        self.stack_top += 1;
        Ok(())
    }

    fn pop(&mut self) -> Value<'gc> {
        let value = self.stack.pop();
        self.stack_top -= 1;
        value.expect("No value to pop from stack.")
    }

    fn peek(&self, depth: usize) -> InterpretResult<&Value<'gc>> {
        self.stack
            .get(self.stack.len() - 1 - depth)
            .ok_or(ChefError::OutOfBounds)
    }

    // Returns boolean indicating whether the current run is complete
    pub(super) fn run(&mut self, steps: u8) -> InterpretResult<bool> {
        let mut current_frame = self.pop_frame();
        for _ in 0..steps {
            let result = self.do_step(&mut current_frame);
            match result {
                Ok(false) => continue,
                Ok(true) | Err(_) => self.push_frame(current_frame)?,
            };
            return result;
        }
        self.push_frame(current_frame)?;
        Ok(false)
    }

    fn do_step(&mut self, current_frame: &mut CallFrame<'gc>) -> InterpretResult<bool> {
        let operation = current_frame.function().chunk.code[current_frame.frame_ip];
        current_frame.frame_ip += 1;
        #[cfg(feature = "debug_trace")]
        current_function
            .chunk
            .disassemble_instruction(current_frame.frame_ip - 1);
        match &operation {
            Operation::Return => {
                let result = self.pop();
                self.close_upvalues(current_frame.stack_index)?;
                if self.frames.len() == 0 {
                    return Ok(true);
                }
                // Unwind the current call frame from the stack.
                self.stack.truncate(current_frame.stack_index);
                *current_frame = self.pop_frame();
                self.push(result)?;
            }
            Operation::Constant(index) => {
                let value = read_constant(current_frame.function(), *index)?;
                self.push(value)?
            }
            Operation::Negate => {
                let constant = self.stack.last_mut().unwrap();
                constant.negate()?
            }
            Operation::Add => {
                let (b, mut a) = (self.pop(), self.pop());
                match (a, b) {
                    (Value::String(a), Value::String(b)) => {
                        let mut root = a.deref().clone();
                        root.push_str(&b);
                        self.push(Value::String(Gc::new(self.mc, root)))?;
                    }
                    _ => {
                        a.add_assign(b)?;
                        self.push(a)?
                    }
                }
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
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                let constant = self.pop();
                self.identifiers.insert(name.deref().clone(), constant);
            }
            Operation::GetGlobal(index) => {
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                let constant = self
                    .identifiers
                    .get(name.deref())
                    .ok_or(ChefError::UndefinedVariable(name.deref().clone()))?;
                self.push(*constant)?
            }
            Operation::SetGlobal(index) => {
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                let constant = self.peek(0)?;
                self.identifiers
                    .insert(name.deref().clone(), *constant)
                    .ok_or(ChefError::UndefinedVariable(name.deref().clone()))?;
            }
            Operation::GetLocal(frame_index) => {
                let stack_index = current_frame.stack_index + *frame_index as usize;
                let value = self.stack[stack_index];
                self.push(value)?
            }
            Operation::SetLocal(frame_index) => {
                let stack_index = current_frame.stack_index + *frame_index as usize;
                let replacement_value = self.peek(0)?;
                self.stack[stack_index] = *replacement_value;
            }
            Operation::JumpIfFalse(jump) => {
                let value = self.peek(0)?;
                if value.falsey() {
                    current_frame.frame_ip += *jump as usize;
                }
            }
            Operation::Jump(jump) => current_frame.frame_ip += *jump as usize,
            Operation::Loop(offset) => current_frame.frame_ip -= (*offset + 1) as usize,
            Operation::Call(argument_count) => {
                if let Some(call_frame) = self.call_value(*argument_count)? {
                    self.push_frame(*current_frame)?;
                    *current_frame = call_frame;
                }
            }
            Operation::Closure(index) => {
                let Value::Function(function) = read_constant(current_frame.function(), *index)?
                else {
                    return Err(ChefError::ConstantFunctionNotFound);
                };
                let (upvalue_count, function) = (function.upvalue_count, function);
                let mut closure_object = ClosureObject::new(upvalue_count, function);
                for _ in 0..upvalue_count {
                    let Operation::ClosureIsLocalByte(is_local) =
                        current_frame.function().chunk.code[current_frame.frame_ip]
                    else {
                        return Err(ChefError::ClosureOpcode);
                    };
                    current_frame.frame_ip += 1;
                    let Operation::ClosureIndexByte(index) =
                        current_frame.function().chunk.code[current_frame.frame_ip]
                    else {
                        return Err(ChefError::ClosureOpcode);
                    };
                    current_frame.frame_ip += 1;
                    let upvalue = if is_local {
                        let upvalue_slot = current_frame.stack_index + index as usize;
                        self.capture_upvalue(upvalue_slot)
                    } else {
                        current_frame.closure().upvalues[index as usize]
                    };
                    closure_object.upvalues.push(upvalue);
                }
                self.push(Value::Closure(Gc::new(self.mc, closure_object)))?
            }
            Operation::GetUpvalue(upvalue_slot) => {
                let slot = *upvalue_slot as usize;
                let upvalue = current_frame.closure().upvalues[slot];
                let value = match &*upvalue.borrow() {
                    UpvalueObject::Open(value_slot) => self.stack[*value_slot],
                    UpvalueObject::Closed(value) => *value,
                };
                self.push(value)?
            }
            Operation::SetUpvalue(upvalue_slot) => {
                let slot = *upvalue_slot as usize;
                let replacement_value = self.peek(0)?;
                let upvalue = current_frame.closure().upvalues[slot];
                let mut upvalue_borrow = upvalue.borrow_mut(self.mc);
                match &mut *upvalue_borrow {
                    UpvalueObject::Open(value_slot) => self.stack[*value_slot] = *replacement_value,
                    UpvalueObject::Closed(value) => *value = *replacement_value,
                };
            }
            Operation::CloseUpvalue => {
                self.close_upvalues(self.stack.len() - 1)?;
                self.pop();
            }
            Operation::ClosureIsLocalByte(_) => unreachable!(),
            Operation::ClosureIndexByte(_) => unreachable!(),
            Operation::Class(index) => {
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantClassNotFound);
                };
                let class = Gc::new(self.mc, RefLock::new(ClassObject::new(&name)));
                self.push(Value::Class(class))?
            }
            Operation::GetProperty(index) => {
                let Value::Instance(instance) = self.peek(0)? else {
                    return Err(ChefError::InstanceGetProperty);
                };
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                match instance.borrow().fields.get(name.deref()) {
                    Some(value) => {
                        self.pop();
                        self.push(*value)?;
                    }
                    None => {
                        self.bind_method(name, (*instance.borrow()).class)?;
                    }
                };
            }
            Operation::SetProperty(index) => {
                let Value::Instance(instance) = *self.peek(1)? else {
                    return Err(ChefError::InstanceSetProperty);
                };
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                let value = self.pop();
                instance
                    .borrow_mut(self.mc)
                    .fields
                    .insert(name.deref().clone(), value);
                self.pop();
                self.push(value)?
            }
            Operation::Method(index) => {
                let Value::String(name) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                self.define_method(name)?;
            }
            Operation::Invoke(index, argument_count) => {
                let Value::String(method) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                if let Some(call_frame) = self.invoke(method, *argument_count)? {
                    self.push_frame(*current_frame)?;
                    *current_frame = call_frame;
                }
            }
            Operation::Inherit => {
                let Value::Class(superclass) = self.peek(1)? else {
                    return Err(ChefError::ConstantSuperclassNotFound);
                };
                let Value::Class(subclass) = self.peek(0)? else {
                    return Err(ChefError::ConstantClassNotFound);
                };
                for (name, method) in superclass.borrow().methods.iter() {
                    subclass
                        .borrow_mut(self.mc)
                        .methods
                        .insert(name.clone(), *method);
                }
                self.pop();
            }
            Operation::GetSuper(index) => {
                let Value::String(method) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                let Value::Class(superclass) = self.pop() else {
                    return Err(ChefError::ConstantSuperclassNotFound);
                };
                self.bind_method(method, superclass)?;
            }
            Operation::SuperInvoke(index, argument_count) => {
                let Value::String(method) = read_constant(current_frame.function(), *index)? else {
                    return Err(ChefError::ConstantStringNotFound);
                };
                let Value::Class(superclass) = self.pop() else {
                    return Err(ChefError::ConstantSuperclassNotFound);
                };
                let call_frame = self.invoke_from_class(superclass, method, *argument_count)?;
                self.push_frame(*current_frame)?;
                *current_frame = call_frame;
            }
        };
        Ok(false)
    }

    fn invoke(
        &mut self,
        method: Gc<'gc, String>,
        argument_count: u8,
    ) -> InterpretResult<Option<CallFrame<'gc>>> {
        let Value::Instance(receiver) = *self.peek(argument_count as usize)? else {
            return Err(ChefError::InstanceInvoke);
        };
        if let Some(field_value) = receiver.borrow().fields.get(method.deref()) {
            let index = self.stack.len() - argument_count as usize - 1;
            self.stack[index] = *field_value;
            self.call_value(argument_count)
        } else {
            let call_frame =
                self.invoke_from_class(receiver.borrow().class, method, argument_count)?;
            Ok(Some(call_frame))
        }
    }

    fn invoke_from_class(
        &mut self,
        class: Gc<'gc, RefLock<ClassObject<'gc>>>,
        name: Gc<'gc, String>,
        argument_count: u8,
    ) -> InterpretResult<CallFrame<'gc>> {
        let method = *class
            .borrow()
            .methods
            .get(name.deref())
            .ok_or(ChefError::UndefinedProperty(name.deref().clone()))?;
        self.call(method, argument_count)
    }

    fn define_method(&mut self, name: Gc<'gc, String>) -> InterpretResult<()> {
        let Value::Closure(closure) = self.pop() else {
            return Err(ChefError::ConstantClosureNotFound);
        };
        let Value::Class(class) = self.peek(0)? else {
            return Err(ChefError::ConstantClassNotFound);
        };
        class
            .borrow_mut(self.mc)
            .add_method(name.deref().clone(), closure);
        Ok(())
    }

    fn close_upvalues(&mut self, from: usize) -> InterpretResult<()> {
        for upvalue in self.upvalues.iter() {
            let slot = match *upvalue.borrow() {
                UpvalueObject::Open(value_slot) => match value_slot >= from {
                    true => value_slot,
                    false => continue,
                },
                UpvalueObject::Closed(_) => continue,
            };
            let value = self.stack.get(slot).ok_or(ChefError::OutOfBounds)?;
            *upvalue.borrow_mut(self.mc) = UpvalueObject::Closed(*value);
        }
        self.upvalues.retain(|upvalue| match *upvalue.borrow() {
            UpvalueObject::Open(_) => true,
            UpvalueObject::Closed(_) => false,
        });
        Ok(())
    }

    fn capture_upvalue(&mut self, stack_index: usize) -> Gc<'gc, RefLock<UpvalueObject<'gc>>> {
        for upvalue in self.upvalues.iter().rev() {
            match *upvalue.borrow() {
                UpvalueObject::Open(slot) => {
                    if slot == stack_index {
                        return *upvalue;
                    }
                }
                UpvalueObject::Closed(_) => continue,
            }
        }
        let upvalue = UpvalueObject::new(stack_index);
        let upvalue = Gc::new(self.mc, RefLock::new(upvalue));
        self.upvalues.push(upvalue);
        upvalue
    }

    fn call_value(&mut self, argument_count: u8) -> InterpretResult<Option<CallFrame<'gc>>> {
        let callee = self.peek(argument_count as usize)?.clone();
        match callee {
            Value::NativeFunction(function) => {
                let result = (function)(argument_count, self.stack_top - argument_count as usize);
                self.stack.truncate(self.stack.len() - 1);
                self.push(result)?;
                Ok(None)
            }
            Value::Closure(closure) => {
                let call_frame = self.call(closure, argument_count)?;
                Ok(Some(call_frame))
            }
            Value::Class(class) => {
                let instance = InstanceObject::new(class);
                let instance = Value::Instance(Gc::new(self.mc, RefLock::new(instance)));
                let index = self.stack.len() - argument_count as usize - 1;
                self.stack[index] = instance;
                if let Some(closure) = class.borrow().methods.get(INIT_STRING) {
                    let call_frame = self.call(*closure, argument_count)?;
                    Ok(Some(call_frame))
                } else if argument_count != 0 {
                    Err(ChefError::ClassArguments(argument_count))
                } else {
                    Ok(None)
                }
            }
            Value::BoundMethod(bound_method) => {
                let closure = bound_method.closure;
                let index = self.stack.len() - argument_count as usize - 1;
                self.stack[index] = Value::Instance(bound_method.receiver);
                let call_frame = self.call(closure, argument_count)?;
                Ok(Some(call_frame))
            }
            _ => Err(ChefError::InvalidCallee),
        }
    }

    pub(super) fn call(
        &mut self,
        closure: Gc<'gc, ClosureObject<'gc>>,
        argument_count: u8,
    ) -> InterpretResult<CallFrame<'gc>> {
        let function = closure.function;
        if function.arity != argument_count {
            return Err(ChefError::FunctionArity(function.arity, argument_count));
        }
        Ok(CallFrame {
            closure,
            stack_index: self.stack.len() - argument_count as usize - 1,
            frame_ip: 0,
        })
    }

    fn bind_method(
        &mut self,
        name: Gc<'gc, String>,
        class: Gc<'gc, RefLock<ClassObject<'gc>>>,
    ) -> InterpretResult<()> {
        let closure = *class
            .borrow()
            .methods
            .get(name.deref())
            .ok_or(ChefError::UndefinedProperty(name.deref().clone()))?;
        let receiver = match self.pop() {
            Value::Instance(instance) => instance,
            _ => return Err(ChefError::NoInstanceOnStack),
        };
        let bound_method = BoundMethod::new(receiver, closure);
        self.push(Value::BoundMethod(Gc::new(self.mc, bound_method)))
    }
}

fn read_constant<'gc, 'a>(
    function: &'a FunctionObject<'gc>,
    index: u8,
) -> InterpretResult<Value<'gc>> {
    let value = function
        .chunk
        .constants
        .get(index as usize)
        .ok_or(ChefError::OutOfBounds)?;
    Ok(*value)
}
