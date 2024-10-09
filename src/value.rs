use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

use crate::objects::Object;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Uninit,
    Nil,
    Number(f64),
    Boolean(bool),
    ObjectValue(Object),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Uninit => write!(f, "uninitialized"),
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::ObjectValue(object) => write!(f, "{object}"),
        }
    }
}

impl Value {
    pub fn negate(&mut self) -> Result<(), String> {
        match self {
            Value::Number(number) => *number = -*number,
            _ => return Err("Operand must be a number.".into()),
        };
        Ok(())
    }

    pub fn add_assign(&mut self, rhs: Value) -> Result<(), String> {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.add_assign(b),
            (Value::ObjectValue(Object::String(a)), Value::ObjectValue(Object::String(b))) => {
                a.data.borrow_mut().push_str(&b.data.borrow())
            }
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn sub_assign(&mut self, rhs: Value) -> Result<(), String> {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.sub_assign(b),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn mul_assign(&mut self, rhs: Value) -> Result<(), String> {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.mul_assign(b),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn div_assign(&mut self, rhs: Value) -> Result<(), String> {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.div_assign(b),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn falsey(&self) -> Result<bool, String> {
        match self {
            Value::Number(_) => Ok(false),
            Value::Boolean(b) => Ok(!b),
            Value::Nil => Ok(true),
            Value::Uninit => Ok(false),
            Value::ObjectValue(_) => Err("Operand for falsiness cannot be an object.".into()),
        }
    }

    pub fn is_equal(&self, rhs: Value) -> bool {
        match (self, rhs) {
            (Value::Nil, Value::Nil) => true,
            (Value::Boolean(a), Value::Boolean(b)) => *a == b,
            (Value::Number(a), Value::Number(b)) => *a == b,
            (Value::ObjectValue(Object::String(a)), Value::ObjectValue(Object::String(b))) => {
                a.data == b.data
            }
            _ => false,
        }
    }

    pub fn is_greater(&self, rhs: Value) -> Result<bool, String> {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Ok(*a > b),
            _ => return Err("Operands must be numbers.".into()),
        }
    }

    pub fn is_less(&self, rhs: Value) -> Result<bool, String> {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Ok(*a < b),
            _ => return Err("Operands must be numbers.".into()),
        }
    }
}
