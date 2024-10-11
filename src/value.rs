use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};
use std::rc::Rc;

use crate::objects::{ClosureObject, FunctionObject, NativeFunctionObject};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(Rc<FunctionObject>),
    NativeFunction(Rc<NativeFunctionObject>),
    Closure(Rc<ClosureObject>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn format_function_object(function_object: &Rc<FunctionObject>) -> String {
    match function_object.name.is_empty() {
        true => "<script>".into(),
        false => format!("<fn {}>", function_object.name),
    }
}

// TODO: clean up Displays for Value types
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::String(rc) => write!(f, "{}", rc),
            Value::Function(rc) => write!(f, "{}", format_function_object(rc)),
            Value::NativeFunction(rc) => write!(f, "<native fn {}>", rc.name),
            Value::Closure(rc) => write!(f, "{}", rc.function_name),
        }
    }
}

impl Value {
    pub fn negate(&mut self) -> Result<(), String> {
        match self {
            Self::Number(number) => *number = -*number,
            _ => return Err("Operand must be a number.".into()),
        };
        Ok(())
    }

    pub fn add_assign(&mut self, rhs: Self) -> Result<(), String> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.add_assign(b),
            (Self::String(a), Self::String(b)) => a.push_str(b.as_str()),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn sub_assign(&mut self, rhs: Self) -> Result<(), String> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.sub_assign(b),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn mul_assign(&mut self, rhs: Self) -> Result<(), String> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.mul_assign(b),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn div_assign(&mut self, rhs: Self) -> Result<(), String> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => a.div_assign(b),
            _ => return Err("Operands must be numbers.".into()),
        };
        Ok(())
    }

    pub fn falsey(&self) -> Result<bool, String> {
        match self {
            Self::Number(_) => Ok(false),
            Self::Boolean(b) => Ok(!b),
            Self::Nil => Ok(true),
            _ => Err("Invalid operand for falsiness.".into()),
        }
    }

    pub fn is_equal(&self, rhs: Self) -> bool {
        match (self, rhs) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => *a == b,
            (Self::Number(a), Self::Number(b)) => *a == b,
            (Self::String(a), Self::String(b)) => b.eq(a),
            _ => false,
        }
    }

    pub fn is_greater(&self, rhs: Self) -> Result<bool, String> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(*a > b),
            _ => return Err("Operands must be numbers.".into()),
        }
    }

    pub fn is_less(&self, rhs: Self) -> Result<bool, String> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(*a < b),
            _ => return Err("Operands must be numbers.".into()),
        }
    }
}
