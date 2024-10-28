use std::collections::HashMap;
use std::mem::transmute;
use std::rc::Rc;

use crate::chunk::Opcode;
use crate::common::{CALL_FRAMES_MAX_COUNT, STACK_VALUES_MAX_COUNT};
use crate::error::{ChefError, InterpretResult};
use crate::function::Function;
use crate::native_functions::declare_native_functions;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct CallFrame {
    function: Rc<Function>,
    stack_index: usize,
    frame_ip: usize,
}

impl CallFrame {
    fn runtime_error_print(&self) {
        let function = &self.function;
        let line = function.chunk.lines[self.frame_ip - 1];
        match function.name.is_empty() {
            true => eprintln!("[line {line}] in script"),
            false => eprintln!("[line {line}] in {}", function.name),
        }
    }
}

pub struct State {
    frames: [Option<CallFrame>; CALL_FRAMES_MAX_COUNT],
    frame_count: usize,
    stack: [Option<Value>; STACK_VALUES_MAX_COUNT],
    stack_top: usize,
    pub globals: HashMap<String, Value>,
}

const FRAME_ARRAY_REPEAT_VALUE: Option<CallFrame> = None;
const STACK_ARRAY_REPEAT_VALUE: Option<Value> = None;
impl State {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        let native_functions = declare_native_functions(&mut globals);
        Self {
            frames: [FRAME_ARRAY_REPEAT_VALUE; CALL_FRAMES_MAX_COUNT],
            frame_count: 0,
            stack: [STACK_ARRAY_REPEAT_VALUE; STACK_VALUES_MAX_COUNT],
            stack_top: 0,
            globals,
        }
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames[self.frame_count - 1].as_ref().unwrap()
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames[self.frame_count - 1].as_mut().unwrap()
    }

    fn reset(&mut self) {
        self.stack_top = 0;
        self.frame_count = 0;
    }

    pub fn stack_error(&mut self) {
        for frame_count in (0..self.frame_count).rev() {
            let frame = self.frames[frame_count].as_ref().unwrap();
            frame.runtime_error_print()
        }
        self.reset();
    }

    pub fn push_frame(&mut self, frame: CallFrame) -> InterpretResult<()> {
        if self.frame_count == CALL_FRAMES_MAX_COUNT {
            return Err(ChefError::StackOverflow);
        }
        self.frames[self.frame_count] = Some(frame);
        self.frame_count += 1;
        Ok(())
    }

    fn pop_frame(&mut self) -> CallFrame {
        self.frame_count -= 1;
        std::mem::replace(&mut self.frames[self.frame_count], None).unwrap()
    }

    pub fn push(&mut self, value: Value) -> InterpretResult<()> {
        if self.stack_top == STACK_VALUES_MAX_COUNT {
            return Err(ChefError::StackOverflow);
        }
        self.stack[self.stack_top] = Some(value);
        self.stack_top += 1;
        Ok(())
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        std::mem::replace(&mut self.stack[self.stack_top], None).unwrap()
    }

    fn peek(&self, depth: usize) -> &Value {
        self.stack[self.stack_top - 1 - depth].as_ref().unwrap()
    }

    pub fn run(&mut self) -> InterpretResult<()> {
        loop {
            let byte = self.read_byte();
            #[cfg(feature = "debug_trace")]
            self.current_frame()
                .function
                .chunk
                .disassemble_instruction(self.current_frame().frame_ip - 1);
            let opcode: Opcode = unsafe { transmute(byte) };
            match opcode {
                Opcode::Return => {
                    let result = self.pop();
                    let frame = self.pop_frame();
                    if self.frame_count == 0 {
                        return Ok(());
                    }
                    self.stack_top = frame.stack_index;
                    self.push(result)?;
                }
                Opcode::Constant => self.op_constant()?,
                Opcode::Negate => self.op_negate()?,
                Opcode::Add => self.op_add()?,
                Opcode::Subtract => self.op_subtract()?,
                Opcode::Multiply => self.op_multiply()?,
                Opcode::Divide => self.op_divide()?,
                Opcode::Nil => self.op_nil()?,
                Opcode::True => self.op_true()?,
                Opcode::False => self.op_false()?,
                Opcode::Not => self.op_not()?,
                Opcode::Equal => self.op_equal()?,
                Opcode::Greater => self.op_greater()?,
                Opcode::Less => self.op_less()?,
                Opcode::Print => self.op_print(),
                Opcode::Pop => drop(self.pop()),
                Opcode::DefineGlobal => self.op_define_global()?,
                Opcode::GetGlobal => self.op_get_global()?,
                Opcode::SetGlobal => self.op_set_global()?,
                Opcode::GetLocal => self.op_get_local()?,
                Opcode::SetLocal => self.op_set_local(),
                Opcode::JumpIfFalse => self.op_jump_if_false(),
                Opcode::Jump => self.op_jump(),
                Opcode::Loop => self.op_loop(),
                Opcode::Call => self.op_call()?,
                Opcode::Function => self.op_function()?,
            };
        }
    }

    fn op_constant(&mut self) -> InterpretResult<()> {
        let constant_index = self.read_byte();
        let value = self.read_constant(constant_index)?;
        self.push(value)?;
        Ok(())
    }

    fn op_negate(&mut self) -> InterpretResult<()> {
        let mut constant = self.pop();
        constant.negate()?;
        self.push(constant)?;
        Ok(())
    }

    fn op_add(&mut self) -> InterpretResult<()> {
        let (b, mut a) = (self.pop(), self.pop());
        match (a.clone(), &b) {
            (Value::String(mut a), Value::String(b)) => {
                a.push_str(&b);
                self.push(Value::String(a.to_string()))?;
            }
            _ => {
                a.add_assign(b)?;
                self.push(a)?;
            }
        }
        Ok(())
    }

    fn op_subtract(&mut self) -> InterpretResult<()> {
        let (b, mut a) = (self.pop(), self.pop());
        a.sub_assign(b)?;
        self.push(a)?;
        Ok(())
    }

    fn op_multiply(&mut self) -> InterpretResult<()> {
        let (b, mut a) = (self.pop(), self.pop());
        a.mul_assign(b)?;
        self.push(a)?;
        Ok(())
    }

    fn op_divide(&mut self) -> InterpretResult<()> {
        let (b, mut a) = (self.pop(), self.pop());
        a.div_assign(b)?;
        self.push(a)?;
        Ok(())
    }

    fn op_nil(&mut self) -> InterpretResult<()> {
        self.push(Value::Nil)?;
        Ok(())
    }

    fn op_true(&mut self) -> InterpretResult<()> {
        self.push(Value::Boolean(true))?;
        Ok(())
    }

    fn op_false(&mut self) -> InterpretResult<()> {
        self.push(Value::Boolean(false))?;
        Ok(())
    }

    fn op_not(&mut self) -> InterpretResult<()> {
        let constant = self.pop();
        let result = constant.falsey();
        self.push(Value::Boolean(result))?;
        Ok(())
    }

    fn op_equal(&mut self) -> InterpretResult<()> {
        let (b, a) = (self.pop(), self.pop());
        let result = a.is_equal(b);
        self.push(Value::Boolean(result))?;
        Ok(())
    }

    fn op_greater(&mut self) -> InterpretResult<()> {
        let (b, a) = (self.pop(), self.pop());
        let result = a.is_greater(b)?;
        self.push(Value::Boolean(result))?;
        Ok(())
    }

    fn op_less(&mut self) -> InterpretResult<()> {
        let (b, a) = (self.pop(), self.pop());
        let result = a.is_less(b)?;
        self.push(Value::Boolean(result))?;
        Ok(())
    }

    fn op_print(&mut self) {
        let constant = self.pop();
        println!("{constant}");
    }

    fn op_loop(&mut self) {
        let offset = self.read_u16();
        self.current_frame_mut().frame_ip -= offset as usize;
    }

    fn op_jump(&mut self) {
        let offset = self.read_u16();
        self.current_frame_mut().frame_ip += offset as usize
    }

    fn op_jump_if_false(&mut self) {
        let offset = self.read_u16();
        let value = self.peek(0);
        if value.falsey() {
            self.current_frame_mut().frame_ip += offset as usize;
        }
    }

    fn op_define_global(&mut self) -> InterpretResult<()> {
        let constant_index = self.read_byte();
        let Value::String(name) = self.read_constant(constant_index)? else {
            return Err(ChefError::ConstantStringNotFound);
        };
        let constant = self.pop();
        self.globals.insert(name, constant);
        Ok(())
    }

    fn op_get_global(&mut self) -> InterpretResult<()> {
        let constant_index = self.read_byte();
        let Value::String(name) = self.read_constant(constant_index)? else {
            return Err(ChefError::ConstantStringNotFound);
        };
        let constant = self
            .globals
            .get(&name)
            .ok_or_else(|| ChefError::UndefinedVariable(name))?;
        self.push(constant.clone())?;
        Ok(())
    }

    fn op_set_global(&mut self) -> InterpretResult<()> {
        let constant_index = self.read_byte();
        let Value::String(name) = self.read_constant(constant_index)? else {
            return Err(ChefError::ConstantStringNotFound);
        };
        let constant = self.peek(0);
        self.globals
            .insert(name.clone(), constant.clone())
            .ok_or_else(|| ChefError::UndefinedVariable(name))?;
        Ok(())
    }

    fn op_get_local(&mut self) -> InterpretResult<()> {
        let frame_index = self.read_byte();
        let stack_index = self.current_frame().stack_index + frame_index as usize;
        let value = self.stack[stack_index].as_ref().unwrap();
        self.push(value.clone())?;
        Ok(())
    }

    fn op_set_local(&mut self) {
        let frame_index = self.read_byte();
        let stack_index = self.current_frame().stack_index + frame_index as usize;
        let replacement_value = self.peek(0);
        self.stack[stack_index] = Some(replacement_value.clone());
    }

    fn op_call(&mut self) -> InterpretResult<()> {
        let argument_count = self.read_byte();
        self.call(argument_count)
    }

    fn op_function(&mut self) -> InterpretResult<()> {
        let constant_index = self.read_byte();
        let value @ Value::Function(_) = self.read_constant(constant_index)? else {
            return Err(ChefError::ConstantFunctionNotFound);
        };
        self.push(value)
    }

    pub fn call(&mut self, argument_count: u8) -> InterpretResult<()> {
        let callee = self.peek(argument_count as usize);
        match callee {
            Value::NativeFunction(function) => {
                let result = function(argument_count, self.stack_top - argument_count as usize);
                self.stack_top -= 1;
                self.push(result)?;
                Ok(())
            }
            Value::Function(function) => {
                if function.arity != argument_count {
                    return Err(ChefError::FunctionArity(function.arity, argument_count));
                }
                self.push_frame(CallFrame {
                    function: Rc::clone(&function),
                    stack_index: self.stack_top - argument_count as usize - 1,
                    frame_ip: 0,
                })?;
                Ok(())
            }
            _ => Err(ChefError::InvalidCallee),
        }
    }

    fn read_constant<'a>(&self, index: u8) -> InterpretResult<Value> {
        let value = self
            .current_frame()
            .function
            .chunk
            .constants
            .get(index as usize)
            .ok_or(ChefError::OutOfBounds)?;
        Ok(value.clone())
    }

    fn read_u16(&mut self) -> usize {
        let current_frame = self.current_frame_mut();
        current_frame.frame_ip += 2;
        let byte_1 = current_frame.function.chunk.code[current_frame.frame_ip - 2];
        let byte_2 = current_frame.function.chunk.code[current_frame.frame_ip - 1];
        u16::from_le_bytes([byte_1, byte_2]) as usize
    }

    fn read_byte(&mut self) -> u8 {
        let current_frame = self.current_frame_mut();
        let byte = current_frame.function.chunk.code[current_frame.frame_ip];
        current_frame.frame_ip += 1;
        byte
    }
}
