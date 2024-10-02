use std::fmt::Debug;
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
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

    pub fn falsey(&self) -> bool {
        match self {
            Value::Number(_) => false,
            Value::Boolean(b) => !b,
            Value::Nil => true,
        }
    }

    pub fn is_equal(&self, rhs: Value) -> bool {
        match (self, rhs) {
            (Value::Nil, Value::Nil) => true,
            (Value::Boolean(a), Value::Boolean(b)) => *a == b,
            (Value::Number(a), Value::Number(b)) => *a == b,
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
