use std::mem::transmute;

use crate::code::{Code, Opcode};
use crate::common::{CALL_FRAMES_MAX_COUNT, STACK_VALUES_MAX_COUNT};
use crate::error::{ChefError, InterpretResult};
use crate::value::Value;

#[derive(Debug, Default, Clone)]
pub struct CallFrame {
    pub name: String,
    pub line: usize,
    pub stack_index: usize,
    pub continuation_ip: usize,
}

pub struct State {
    ip: usize,
    code: Code,
    frames: [Option<CallFrame>; CALL_FRAMES_MAX_COUNT],
    frame_count: usize,
    stack: [Option<Value>; STACK_VALUES_MAX_COUNT],
    stack_top: usize,
}

const FRAME_ARRAY_REPEAT_VALUE: Option<CallFrame> = None;
const STACK_ARRAY_REPEAT_VALUE: Option<Value> = None;
impl State {
    pub fn new(code: Code) -> Self {
        Self {
            ip: 0,
            code,
            frames: [FRAME_ARRAY_REPEAT_VALUE; CALL_FRAMES_MAX_COUNT],
            frame_count: 0,
            stack: [STACK_ARRAY_REPEAT_VALUE; STACK_VALUES_MAX_COUNT],
            stack_top: 0,
        }
    }

    fn reset(&mut self) {
        self.stack_top = 0;
        self.frame_count = 0;
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames[self.frame_count - 1].as_mut().unwrap()
    }

    pub fn stack_error(&mut self) {
        self.current_frame_mut().line = self.code.lines[self.ip];
        for frame_count in (0..self.frame_count).rev() {
            let frame = self.frames[frame_count].as_ref().unwrap();
            let line = frame.line;
            match frame.name.is_empty() {
                true => eprintln!("[line {line}] in script"),
                false => eprintln!("[line {line}] in {}", frame.name),
            }
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
        self.frames[self.frame_count].take().unwrap()
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
        self.stack[self.stack_top].take().unwrap()
    }

    fn peek(&self, depth: usize) -> &Value {
        self.stack[self.stack_top - 1 - depth].as_ref().unwrap()
    }

    pub fn run(&mut self) -> InterpretResult<()> {
        loop {
            let byte = self.read_byte();
            #[cfg(feature = "debug_trace")]
            self.code.disassemble_instruction(self.ip - 1);
            let opcode: Opcode = unsafe { transmute(byte) };
            match opcode {
                Opcode::Return => {
                    let result = self.pop();
                    let frame = self.pop_frame();
                    if self.frame_count == 0 {
                        return Ok(());
                    }
                    self.stack_top = frame.stack_index;
                    self.ip = frame.continuation_ip;
                    self.pop();
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
                Opcode::GetLocal => self.op_get_local()?,
                Opcode::SetLocal => self.op_set_local(),
                Opcode::JumpIfFalse => self.op_jump_if_false(),
                Opcode::Jump => self.op_jump(),
                Opcode::Loop => self.op_loop(),
                Opcode::Call => self.op_call()?,
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
                a.push_str(b);
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
        self.ip -= offset;
    }

    fn op_jump(&mut self) {
        let offset = self.read_u16();
        self.ip += offset
    }

    fn op_jump_if_false(&mut self) {
        let offset = self.read_u16();
        let value = self.peek(0);
        if value.falsey() {
            self.ip += offset;
        }
    }

    fn op_get_local(&mut self) -> InterpretResult<()> {
        let index = self.read_byte();
        let frame_pops = self.read_byte();
        let frame = self.frames[self.frame_count - 1 - frame_pops as usize]
            .as_ref()
            .unwrap();
        let stack_index = frame.stack_index + index as usize;
        let value = self.stack[stack_index].as_ref().unwrap();
        self.push(value.clone())?;
        Ok(())
    }

    fn op_set_local(&mut self) {
        let index = self.read_byte();
        let frame_pops = self.read_byte();
        let frame = self.frames[self.frame_count - 1 - frame_pops as usize]
            .as_ref()
            .unwrap();
        let stack_index = frame.stack_index + index as usize;
        let replacement_value = self.peek(0);
        self.stack[stack_index] = Some(replacement_value.clone());
    }

    fn op_call(&mut self) -> InterpretResult<()> {
        let argument_count = self.read_byte();
        self.call(argument_count)
    }

    pub fn call(&mut self, argument_count: u8) -> InterpretResult<()> {
        let callee = self.peek(argument_count as usize).clone();
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
                self.current_frame_mut().line = self.code.lines[self.ip];
                self.push_frame(CallFrame {
                    name: function.name.clone(),
                    line: 0,
                    stack_index: self.stack_top - argument_count as usize,
                    continuation_ip: self.ip,
                })?;
                self.ip = function.ip_start;
                Ok(())
            }
            _ => Err(ChefError::InvalidCallee),
        }
    }

    fn read_constant(&self, index: u8) -> InterpretResult<Value> {
        let value = self
            .code
            .constants
            .get(index as usize)
            .ok_or(ChefError::OutOfBounds)?;
        Ok(value.clone())
    }

    fn read_u16(&mut self) -> usize {
        self.ip += 2;
        let byte_1 = self.code.bytes[self.ip - 2];
        let byte_2 = self.code.bytes[self.ip - 1];
        u16::from_le_bytes([byte_1, byte_2]) as usize
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.code.bytes[self.ip];
        self.ip += 1;
        byte
    }
}
