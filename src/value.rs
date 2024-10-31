use crate::common::print_function;
use crate::error::{ChefError, InterpretResult};
use crate::native_functions::NativeFunction;
use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub arity: u8,
    pub ip_start: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(Function),
    NativeFunction(NativeFunction),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::String(string) => write!(f, "{string}"),
            Value::Function(function) => write!(f, "{}", print_function(&function.name)),
            Value::NativeFunction(_) => write!(f, "<native fn>"),
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
