use std::fmt::Debug;
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
}

#[derive(PartialEq, Eq)]
pub enum ValueOperationResult {
    Ok,
    Error,
}

impl Value {
    pub fn negate(&mut self) -> ValueOperationResult {
        match self {
            Value::Number(number) => *number = -*number,
            _ => return ValueOperationResult::Error,
        };
        ValueOperationResult::Ok
    }

    pub fn add(&mut self, rhs: Value) -> ValueOperationResult {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.add_assign(b),
            _ => return ValueOperationResult::Error,
        };
        ValueOperationResult::Ok
    }

    pub fn sub(&mut self, rhs: Value) -> ValueOperationResult {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.sub_assign(b),
            _ => return ValueOperationResult::Error,
        };
        ValueOperationResult::Ok
    }

    pub fn mul(&mut self, rhs: Value) -> ValueOperationResult {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.mul_assign(b),
            _ => return ValueOperationResult::Error,
        };
        ValueOperationResult::Ok
    }

    pub fn div(&mut self, rhs: Value) -> ValueOperationResult {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => a.div_assign(b),
            _ => return ValueOperationResult::Error,
        };
        ValueOperationResult::Ok
    }
}
