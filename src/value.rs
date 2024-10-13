use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};
use std::rc::{Rc, Weak};

use crate::objects::{
    ClassObject, ClosureObject, FunctionObject, InstanceObject, NativeFunctionObject,
};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(Rc<FunctionObject>),
    NativeFunction(Rc<NativeFunctionObject>),
    Closure(Rc<ClosureObject>),
    Class(Rc<RefCell<ClassObject>>),
    Instance(Rc<RefCell<InstanceObject>>),
}

#[derive(Debug, Clone)]
pub enum WeakValue {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(Weak<FunctionObject>),
    NativeFunction(Weak<NativeFunctionObject>),
    Closure(Weak<ClosureObject>),
    Class(Weak<RefCell<ClassObject>>),
    Instance(Weak<RefCell<InstanceObject>>),
}

impl Value {
    pub fn downgrade(&self) -> WeakValue {
        match self {
            Value::Nil => WeakValue::Nil,
            Value::Number(number) => WeakValue::Number(*number),
            Value::Boolean(boolean) => WeakValue::Boolean(*boolean),
            Value::String(string) => WeakValue::String(string.clone()),
            Value::Function(rc) => WeakValue::Function(Rc::downgrade(rc)),
            Value::NativeFunction(rc) => WeakValue::NativeFunction(Rc::downgrade(rc)),
            Value::Closure(rc) => WeakValue::Closure(Rc::downgrade(rc)),
            Value::Class(rc) => WeakValue::Class(Rc::downgrade(rc)),
            Value::Instance(rc) => WeakValue::Instance(Rc::downgrade(rc)),
        }
    }
}

impl WeakValue {
    pub fn upgrade(&self) -> Value {
        match self {
            WeakValue::Nil => Value::Nil,
            WeakValue::Number(number) => Value::Number(*number),
            WeakValue::Boolean(boolean) => Value::Boolean(*boolean),
            WeakValue::String(string) => Value::String(string.clone()),
            WeakValue::Function(weak) => Value::Function(weak.upgrade().unwrap()), // TODO: proper error handling, this error can and should make it to a user e.g. if they try to print a memnber whose variable dropped out of scope
            WeakValue::NativeFunction(weak) => Value::NativeFunction(weak.upgrade().unwrap()),
            WeakValue::Closure(weak) => Value::Closure(weak.upgrade().unwrap()),
            WeakValue::Class(weak) => Value::Class(weak.upgrade().unwrap()),
            WeakValue::Instance(weak) => Value::Instance(weak.upgrade().unwrap()),
        }
    }
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

// TODO: clean up Displays for Value types (e.g. looking through weak types)
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
            Value::Class(rc) => write!(f, "class {}", rc.borrow().name),
            Value::Instance(_rc) => write!(f, "class instance"),
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
