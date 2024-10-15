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
            Value::Class(rc) => write!(f, "{}", rc.borrow().name),
            Value::Instance(_rc) => write!(f, "class instance"),
            Value::BoundMethod(_rc) => write!(f, "bound method"),
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

    pub fn falsey(&self) -> InterpretResult<bool> {
        match self {
            Self::Number(_) => Ok(false),
            Self::Boolean(b) => Ok(!b),
            Self::Nil => Ok(true),
            _ => Err(RuntimeError::ValueFalsinessOperation),
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
