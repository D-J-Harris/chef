use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::{chunk::Chunk, value::Value};

#[derive(Debug, Clone)]
pub enum Object {
    String(Rc<ObjectString>),
    Function(Rc<FunctionObject>),
    NativeFunction(Rc<NativeFunctionObject>),
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::String(rc) => write!(f, "{}", rc.data.borrow()),
            Object::Function(rc) => match rc.name.is_empty() {
                true => write!(f, "<script>"),
                false => write!(f, "<fn {}>", rc.name),
            },
            Object::NativeFunction(rc) => write!(f, "<native fn {}>", rc.name),
        }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[derive(Debug, Default)]
pub struct ObjectCommon {
    pub next: Option<Rc<Object>>,
}

#[derive(Debug)]
pub struct ObjectString {
    pub common: ObjectCommon,
    pub data: RefCell<String>,
}

impl ObjectString {
    pub fn new(data: &str) -> Self {
        Self {
            common: ObjectCommon::default(),
            data: RefCell::new(data.into()),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FunctionKind {
    Script,
    Function,
}

#[derive(Debug)]
pub struct FunctionObject {
    pub common: ObjectCommon,
    pub kind: FunctionKind,
    pub name: String,
    pub arity: u8,
    pub chunk: Chunk,
}

impl FunctionObject {
    pub fn new(name: &str, kind: FunctionKind) -> Self {
        Self {
            common: ObjectCommon::default(),
            chunk: Chunk::new(),
            name: name.into(),
            kind,
            arity: 0,
        }
    }
}

pub type NativeFunction = fn(arg_count: u8, ip: usize) -> Value;

#[derive(Debug)]
pub struct NativeFunctionObject {
    pub common: ObjectCommon,
    pub name: String,
    pub function: NativeFunction,
}

impl NativeFunctionObject {
    pub fn new(name: &str, function: NativeFunction) -> Self {
        Self {
            common: ObjectCommon::default(),
            name: name.into(),
            function,
        }
    }
}
