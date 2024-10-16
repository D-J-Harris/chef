use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::chunk::Operation;
use crate::common::{FRAMES_MAX_COUNT, INIT_STRING, STACK_VALUES_MAX_COUNT};
use crate::error::{InterpretResult, RuntimeError};
use crate::objects::{
    BoundMethod, ClassObject, ClosureObject, FunctionObject, InstanceObject, UpvalueObject,
};
use crate::value::Value;

#[derive(Debug, Clone)]
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

    pub fn stack_error(&mut self) {
        for frame_index in (0..self.frame_count).rev() {
            let frame = self.frames[frame_index]
                .as_ref()
                .expect("Could not find stack frame.");
            frame.runtime_error_print()
        }
        self.reset();
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
        let frame = self.frames[self.frame_count].take();
        frame.expect("No frame to pop from stack.")
    }

    fn read_constant(&self, function: &Rc<FunctionObject>, index: u8) -> InterpretResult<Value> {
        Ok(function
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
        let value = self.stack[self.stack_top].take();
        value.expect("No value to pop from stack.")
    }

    fn current_slot(&self) -> usize {
        self.stack_top - 1
    }

    fn peek(&self, slot: usize) -> InterpretResult<&Value> {
        self.stack
            .get(slot)
            .ok_or(RuntimeError::OutOfBounds)?
            .as_ref()
            .ok_or(RuntimeError::UninitializedStackValue)
    }

    fn peek_mut(&mut self, slot: usize) -> InterpretResult<&mut Value> {
        self.stack
            .get_mut(slot)
            .ok_or(RuntimeError::OutOfBounds)?
            .as_mut()
            .ok_or(RuntimeError::UninitializedStackValue)
    }

    pub fn run(&mut self) -> InterpretResult<()> {
        let mut current_frame = self.pop_frame();
        let mut current_closure = Rc::clone(&current_frame.closure);
        let mut current_function = Rc::clone(&current_closure.function);

        let ret: InterpretResult<()> = (|| {
            loop {
                let operation = current_function.chunk.code[current_frame.ip];
                current_frame.ip += 1;
                #[cfg(feature = "debug_trace")]
                current_function.chunk.disassemble_instruction(current_frame.ip - 1);

                match &operation {
                    Operation::Return => {
                        let result = self.pop();
                        self.close_upvalues(current_frame.slot)?;
                        if self.frame_count == 0 {
                            return Ok(());
                        }
                        // Unwind the current call frame from the stack.
                        loop {
                            drop(self.pop());
                            if self.stack_top == current_frame.slot {
                                break;
                            }
                        }
                        if self.frame_count == 0 {
                            break Ok(());
                        }
                        current_frame = self.pop_frame();
                        current_closure = Rc::clone(&current_frame.closure);
                        current_function = Rc::clone(&current_closure.function);

                        self.push(result)?
                    }
                    Operation::Constant(index) => {
                        let value = self.read_constant(&current_function, *index)?;
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
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        let constant = self.pop();
                        self.identifiers.insert(name, constant);
                    }
                    Operation::GetGlobal(index) => {
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        let constant = self
                            .identifiers
                            .get(&name)
                            .ok_or(RuntimeError::UndefinedVariable(name))?;
                        self.push(constant.clone())?
                    }
                    Operation::SetGlobal(index) => {
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        let constant = self.peek(self.current_slot())?;
                        self.identifiers
                            .insert(name.clone(), constant.clone())
                            .ok_or(RuntimeError::UndefinedVariable(name))?;
                    }
                    Operation::GetLocal(frame_index) => {
                        let slot = current_frame.slot + *frame_index as usize;
                        let value = self.peek(slot)?;
                        self.push(value.clone())?
                    }
                    Operation::SetLocal(frame_index) => {
                        let slot = current_frame.slot + *frame_index as usize;
                        let replacement_value = self.peek(self.current_slot())?;
                        *self.peek_mut(slot)? = replacement_value.clone();
                    }
                    Operation::JumpIfFalse(jump) => {
                        let value = self.peek(self.current_slot())?;
                        if value.falsey() {
                            current_frame.ip += *jump as usize;
                        }
                    }
                    Operation::Jump(jump) => current_frame.ip += *jump as usize,
                    Operation::Loop(offset) => current_frame.ip -= (*offset + 1) as usize,
                    Operation::Call(argument_count) => {
                        if let Some(new_frame) = self.call_value(*argument_count)? {
                            self.push_frame(current_frame.clone())?;
                            current_frame = new_frame;
                            current_closure = Rc::clone(&current_frame.closure);
                            current_function = Rc::clone(&current_closure.function);
                        }
                    }
                    Operation::Closure(index) => {
                        let Value::Function(function) =
                            self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantFunctionNotFound);
                        };
                        let (upvalue_count, function) =
                            (function.upvalue_count, Rc::clone(&function));
                        let mut closure_object =
                            ClosureObject::new(upvalue_count, Rc::clone(&function));
                        for i in 0..upvalue_count {
                            let Operation::ClosureIsLocalByte(is_local) =
                                current_function.chunk.code[current_frame.ip]
                            else {
                                return Err(RuntimeError::ClosureOpcode);
                            };
                            current_frame.ip += 1;
                            let Operation::ClosureIndexByte(index) =
                                current_function.chunk.code[current_frame.ip]
                            else {
                                return Err(RuntimeError::ClosureOpcode);
                            };
                            current_frame.ip += 1;
                            let upvalue = if is_local {
                                let upvalue_slot = current_frame.slot + index as usize;
                                self.capture_upvalue(upvalue_slot)
                            } else {
                                Rc::clone(
                                    current_closure.upvalues[index as usize].as_ref().unwrap(),
                                )
                            };
                            closure_object.upvalues[i].replace(upvalue);
                        }
                        self.push(Value::Closure(Rc::new(closure_object)))?
                    }
                    Operation::GetUpvalue(upvalue_slot) => {
                        let slot = *upvalue_slot as usize;
                        let Some(upvalue) = &current_closure.upvalues[slot] else {
                            return Err(RuntimeError::OutOfBounds);
                        };
                        let value = match &*upvalue.borrow() {
                            UpvalueObject::Open(value_slot) => {
                                let value = self.stack[*value_slot].as_ref().unwrap();
                                value.clone()
                            }
                            UpvalueObject::Closed(value) => value.clone(),
                        };
                        self.push(value)?
                    }
                    Operation::SetUpvalue(upvalue_slot) => {
                        let slot = *upvalue_slot as usize;
                        let replacement_value = self.peek(self.current_slot())?.clone();
                        let Some(ref upvalue) = current_closure.upvalues[slot] else {
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
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantClassNotFound);
                        };
                        let class = Rc::new(RefCell::new(ClassObject::new(&name)));
                        self.push(Value::Class(class))?
                    }
                    Operation::GetProperty(index) => {
                        let Value::Instance(instance) = self.peek(self.current_slot())? else {
                            return Err(RuntimeError::InstanceGetProperty);
                        };
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
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
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
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
                        let Value::String(name) = self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        self.define_method(name)?;
                    }
                    Operation::Invoke(index, argument_count) => {
                        let Value::String(method) =
                            self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        if let Some(new_frame) = self.invoke(&method, *argument_count)? {
                            self.push_frame(current_frame.clone())?;
                            current_frame = new_frame;
                            current_closure = Rc::clone(&current_frame.closure);
                            current_function = Rc::clone(&current_closure.function);
                        }
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
                                .insert(name.clone(), Rc::clone(method));
                        }
                        self.pop();
                    }
                    Operation::GetSuper(index) => {
                        let Value::String(method) =
                            self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        let Value::Class(superclass) = self.pop() else {
                            return Err(RuntimeError::ConstantSuperclassNotFound);
                        };
                        self.bind_method(method, superclass.as_ref())?;
                    }
                    Operation::SuperInvoke(index, argument_count) => {
                        let Value::String(method) =
                            self.read_constant(&current_function, *index)?
                        else {
                            return Err(RuntimeError::ConstantStringNotFound);
                        };
                        let Value::Class(superclass) = self.pop() else {
                            return Err(RuntimeError::ConstantSuperclassNotFound);
                        };
                        let new_frame =
                            self.invoke_from_class(&superclass, &method, *argument_count)?;

                        self.push_frame(current_frame.clone())?;
                        current_frame = new_frame;
                        current_closure = Rc::clone(&current_frame.closure);
                        current_function = Rc::clone(&current_closure.function);
                    }
                }
            }
        })();
        // final frame push to make sure stack trace print works, before propagating error
        self.push_frame(current_frame.clone())?;
        ret?;
        Ok(())
    }

    fn invoke(&mut self, method: &str, argument_count: u8) -> InterpretResult<Option<CallFrame>> {
        let Value::Instance(receiver) = self.peek(self.current_slot() - argument_count as usize)?
        else {
            return Err(RuntimeError::InstanceInvoke);
        };
        if let Some(field_value) = Rc::clone(receiver).borrow().fields.get(method) {
            let value = field_value.try_into()?;
            self.stack[self.current_slot() - argument_count as usize] = Some(value);
            return self.call_value(argument_count);
        }
        let call_frame =
            self.invoke_from_class(&Rc::clone(receiver).borrow().class, method, argument_count)?;
        Ok(Some(call_frame))
    }

    fn invoke_from_class(
        &mut self,
        class: &Rc<RefCell<ClassObject>>,
        name: &str,
        argument_count: u8,
    ) -> InterpretResult<CallFrame> {
        let class_borrow = class.borrow();
        let method = class_borrow
            .methods
            .get(name)
            .ok_or(RuntimeError::UndefinedProperty(name.into()))?;
        let call_frame = self.call(Rc::clone(method), argument_count)?;
        Ok(call_frame)
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
                        return Rc::clone(upvalue);
                    }
                }
                UpvalueObject::Closed(_) => continue,
            }
        }
        let upvalue = Rc::new(RefCell::new(UpvalueObject::new(value_slot)));
        self.open_upvalues.push(Rc::clone(&upvalue));
        upvalue
    }

    fn call_value(&mut self, argument_count: u8) -> InterpretResult<Option<CallFrame>> {
        let callee = self
            .peek(self.current_slot() - argument_count as usize)?
            .clone();
        match callee {
            Value::NativeFunction(function) => {
                let result =
                    (function.function)(argument_count, self.stack_top - argument_count as usize);
                self.stack_top -= argument_count as usize + 1;
                self.push(result)?;
                Ok(None)
            }
            Value::Closure(closure) => {
                let call_frame = self.call(Rc::clone(&closure), argument_count)?;
                Ok(Some(call_frame))
            }
            Value::Class(class) => {
                let instance = Value::Instance(Rc::new(RefCell::new(InstanceObject::new(
                    Rc::clone(&class),
                ))));
                self.stack[self.current_slot() - argument_count as usize].replace(instance);
                if let Some(closure) = class.borrow().methods.get(INIT_STRING) {
                    let call_frame = self.call(Rc::clone(closure), argument_count)?;
                    Ok(Some(call_frame))
                } else if argument_count != 0 {
                    Err(RuntimeError::ClassArguments(argument_count))
                } else {
                    Ok(None)
                }
            }
            Value::BoundMethod(bound_method) => {
                let closure = bound_method.closure;
                self.stack[self.current_slot() - argument_count as usize] =
                    Some(Value::Instance(Rc::clone(&bound_method.receiver)));
                let call_frame = self.call(closure, argument_count)?;
                Ok(Some(call_frame))
            }
            _ => Err(RuntimeError::InvalidCallee),
        }
    }

    fn call(
        &mut self,
        closure: Rc<ClosureObject>,
        argument_count: u8,
    ) -> InterpretResult<CallFrame> {
        let function = &closure.function;
        if function.arity != argument_count {
            return Err(RuntimeError::FunctionArity(function.arity, argument_count));
        }
        let frame = CallFrame {
            slot: self.stack_top - (argument_count as usize + 1),
            closure,
            ip: 0,
        };
        Ok(frame)
    }

    fn bind_method(&mut self, name: String, class: &RefCell<ClassObject>) -> InterpretResult<()> {
        let class = class.borrow();
        let closure = class
            .methods
            .get(&name)
            .ok_or(RuntimeError::UndefinedProperty(name))?;
        let receiver = match self.pop() {
            Value::Instance(instance) => instance,
            _ => return Err(RuntimeError::NoInstanceOnStack),
        };
        let bound_method = BoundMethod::new(Rc::clone(&receiver), Rc::clone(closure));
        self.push(Value::BoundMethod(bound_method))
    }

    pub fn interpret(&mut self, function: FunctionObject) -> InterpretResult<()> {
        let function = Rc::new(function);
        self.push(Value::Function(Rc::clone(&function)))?;
        let closure = Rc::new(ClosureObject::new(
            function.upvalue_count,
            Rc::clone(&function),
        ));
        let initial_call_frame = self
            .call(closure, 0)
            .expect("Failed to call top-level script.");
        self.push_frame(initial_call_frame)?;
        let result = self.run();
        if let Err(err) = &result {
            eprintln!("{err}");
            self.stack_error();
        };
        result
    }
}
