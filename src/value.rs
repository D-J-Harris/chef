use crate::common::print_function;
use crate::error::{ChefError, InterpretResult};
use crate::function::Function;
use crate::native_functions::NativeFunction;
use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};
use std::rc::Rc;

pub type FunctionIndex = usize;
pub type NativeFunctionIndex = usize;

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(Rc<Function>),
    NativeFunction(NativeFunction),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::NativeFunction(a), Self::NativeFunction(b)) => a == b,
            (Self::Function(a), Self::Function(b)) => Rc::ptr_eq(&a, &b),
            _ => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::String(string) => write!(f, "{string}"),
            Value::NativeFunction(_) => write!(f, "<native fn>"),
            Value::Function(rc) => write!(f, "{}", print_function(&rc.name)),
        }
    }
}

impl Value {
    pub fn negate(&mut self) -> InterpretResult<()> {
        match self {
            Self::Number(number) => *number = -*number,
            _ => return Err(ChefError::ValueNegationOperation),
        };
        Ok(())
    }

    pub fn add_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.add_assign(b),
            _ => return Err(ChefError::ValueAddOperation),
        };
        Ok(())
    }

    pub fn sub_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.sub_assign(b),
            _ => return Err(ChefError::ValueNumberOnlyOperation),
        };
        Ok(())
    }

    pub fn mul_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.mul_assign(b),
            _ => return Err(ChefError::ValueNumberOnlyOperation),
        };
        Ok(())
    }

    pub fn div_assign(&mut self, rhs: Self) -> InterpretResult<()> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.div_assign(b),
            _ => return Err(ChefError::ValueNumberOnlyOperation),
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
            _ => Err(ChefError::ValueNumberOnlyOperation),
        }
    }

    pub fn is_less(&self, rhs: Self) -> InterpretResult<bool> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(*a < b),
            _ => Err(ChefError::ValueNumberOnlyOperation),
        }
    }
}
