use crate::error::{InterpretResult, RuntimeError};
use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};
use std::rc::{Rc, Weak};

use crate::objects::{
    BoundMethod, ClassObject, ClosureObject, FunctionObject, InstanceObject, NativeFunctionObject,
};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    BoundMethod(BoundMethod),
    Function(Rc<FunctionObject>),
    NativeFunction(Rc<NativeFunctionObject>),
    Closure(Rc<ClosureObject>),
    Class(Rc<RefCell<ClassObject>>),
    Instance(Rc<RefCell<InstanceObject>>),
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(Rc<FunctionObject>),
    NativeFunction(Rc<NativeFunctionObject>),
    Closure(Rc<ClosureObject>),
    Class(Rc<RefCell<ClassObject>>),
    Instance(Weak<RefCell<InstanceObject>>),
    BoundMethod(BoundMethod),
}

impl TryFrom<&FieldValue> for Value {
    type Error = RuntimeError;

    fn try_from(value: &FieldValue) -> Result<Self, Self::Error> {
        Ok(match value {
            FieldValue::Nil => Value::Nil,
            FieldValue::Number(x) => Value::Number(*x),
            FieldValue::Boolean(x) => Value::Boolean(*x),
            FieldValue::String(x) => Value::String(x.clone()),
            FieldValue::Function(rc) => Value::Function(Rc::clone(rc)),
            FieldValue::NativeFunction(rc) => Value::NativeFunction(Rc::clone(rc)),
            FieldValue::Closure(rc) => Value::Closure(Rc::clone(rc)),
            FieldValue::Class(rc) => Value::Class(Rc::clone(rc)),
            FieldValue::Instance(weak) => Value::Instance(
                weak.upgrade()
                    .ok_or(RuntimeError::InstanceReferenceInvalid)?,
            ),
            FieldValue::BoundMethod(bound_method) => Value::BoundMethod(bound_method.clone()),
        })
    }
}

impl From<Value> for FieldValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Nil => FieldValue::Nil,
            Value::Number(x) => FieldValue::Number(x),
            Value::Boolean(x) => FieldValue::Boolean(x),
            Value::String(x) => FieldValue::String(x.clone()),
            Value::Function(rc) => FieldValue::Function(Rc::clone(&rc)),
            Value::NativeFunction(rc) => FieldValue::NativeFunction(Rc::clone(&rc)),
            Value::Closure(rc) => FieldValue::Closure(Rc::clone(&rc)),
            Value::Class(rc) => FieldValue::Class(Rc::clone(&rc)),
            Value::Instance(weak) => FieldValue::Instance(Rc::downgrade(&weak)),
            Value::BoundMethod(bound_method) => FieldValue::BoundMethod(bound_method.clone()),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => *a == *b,
            (Self::Number(a), Self::Number(b)) => *a == *b,
            (Self::String(a), Self::String(b)) => b.eq(a),
            (Self::BoundMethod(a), Self::BoundMethod(b)) => b.eq(a),
            (Self::Class(a), Self::Class(b)) => Rc::ptr_eq(a, b),
            (Self::Closure(a), Self::Closure(b)) => Rc::ptr_eq(a, b),
            (Self::NativeFunction(a), Self::NativeFunction(b)) => Rc::ptr_eq(a, b),
            (Self::Function(a), Self::Function(b)) => Rc::ptr_eq(a, b),
            (Self::Instance(a), Self::Instance(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

fn print_function(name: &str) -> String {
    match name.is_empty() {
        true => "<script>".into(),
        false => format!("<fn {}>", name),
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
            Value::Closure(rc) => write!(f, "{}", print_function(&rc.function.name)),
            Value::Class(rc) => write!(f, "{}", rc.borrow().name),
            Value::Instance(rc) => write!(f, "{} instance", rc.borrow().class.borrow().name),
            Value::BoundMethod(rc) => write!(f, "{}", print_function(&rc.closure.function.name)),
        }
    }
}

impl Value {
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
            _ => return Err(RuntimeError::ValueNumberOnlyOperation),
        }
    }

    pub fn is_less(&self, rhs: Self) -> InterpretResult<bool> {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(*a < b),
            _ => return Err(RuntimeError::ValueNumberOnlyOperation),
        }
    }
}
