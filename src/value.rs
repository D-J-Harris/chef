use gc_arena::lock::RefLock;
use gc_arena::{Collect, Gc};

use crate::common::print_function;
use crate::error::{InterpretResult, RuntimeError};
use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

use crate::objects::{
    BoundMethod, ClassObject, ClosureObject, FunctionObject, InstanceObject, NativeFunctionObject,
};

#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub enum Value<'gc> {
    Nil,
    Number(f64),
    Boolean(bool),
    String(Gc<'gc, String>),
    BoundMethod(BoundMethod<'gc>),
    Closure(Gc<'gc, ClosureObject<'gc>>),
    Function(Gc<'gc, FunctionObject<'gc>>),
    Class(Gc<'gc, RefLock<ClassObject<'gc>>>),
    Instance(Gc<'gc, RefLock<InstanceObject<'gc>>>),
    NativeFunction(Gc<'gc, NativeFunctionObject<'gc>>),
}

impl PartialEq for Value<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => *a == *b,
            (Self::Number(a), Self::Number(b)) => *a == *b,
            (Self::String(a), Self::String(b)) => b.eq(a),
            (Self::BoundMethod(a), Self::BoundMethod(b)) => b.eq(a),
            (Self::Class(a), Self::Class(b)) => std::ptr::eq(a, b),
            (Self::Closure(a), Self::Closure(b)) => std::ptr::eq(a, b),
            (Self::NativeFunction(a), Self::NativeFunction(b)) => std::ptr::eq(a, b),
            (Self::Function(a), Self::Function(b)) => std::ptr::eq(a, b),
            (Self::Instance(a), Self::Instance(b)) => std::ptr::eq(a, b),
            _ => false,
        }
    }
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::String(string) => write!(f, "{string}"),
            Value::NativeFunction(_) => write!(f, "<native fn>"),
            Value::Function(rc) => write!(f, "{}", print_function(&rc.name)),
            Value::Closure(rc) => write!(f, "{}", print_function(&rc.function.name)),
            Value::Class(rc) => write!(f, "{}", rc.borrow().name),
            Value::Instance(rc) => write!(f, "{} instance", rc.borrow().class.borrow().name),
            Value::BoundMethod(rc) => write!(f, "{}", print_function(&rc.closure.function.name)),
        }
    }
}

impl Value<'_> {
    pub fn negate(&mut self) -> InterpretResult<()> {
        match self {
            Self::Number(number) => *number = -*number,
            _ => return Err(RuntimeError::ValueNegationOperation),
        };
        Ok(())
    }

    pub fn add_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.add_assign(b),
            (Self::String(a), Self::String(b)) => a.push_str(b.as_str()),
            _ => return Err(RuntimeError::ValueAddOperation),
        };
        Ok(())
    }

    pub fn sub_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.sub_assign(b),
            _ => return Err(RuntimeError::ValueNumberOnlyOperation),
        };
        Ok(())
    }

    pub fn mul_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.mul_assign(b),
            _ => return Err(RuntimeError::ValueNumberOnlyOperation),
        };
        Ok(())
    }

    pub fn div_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.div_assign(b),
            _ => return Err(RuntimeError::ValueNumberOnlyOperation),
        };
        Ok(())
    }

    pub fn falsey(&self) -> bool {
        match self {
            Self::Boolean(boolean) => !boolean,
            Self::Nil => true,
            _ => false,
        }
    }

    pub fn is_equal(&self, rhs: Self) -> bool {
        rhs.eq(self)
    }

    pub fn is_greater(&self, rhs: Self) -> InterpretResult<bool> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(*a > b),
            _ => Err(RuntimeError::ValueNumberOnlyOperation),
        }
    }

    pub fn is_less(&self, rhs: Self) -> InterpretResult<bool> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(*a < b),
            _ => Err(RuntimeError::ValueNumberOnlyOperation),
        }
    }
}
