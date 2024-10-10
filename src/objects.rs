use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::{chunk::Chunk, value::Value};

#[derive(Debug, Clone)]
pub enum Object {
    String(Rc<ObjectString>),
    Function(Rc<FunctionObject>),
    NativeFunction(Rc<NativeFunctionObject>),
    Closure(Rc<ClosureObject>),
    Upvalue(Rc<UpvalueObject>),
}

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn format_function_object(function_object: &Rc<FunctionObject>) -> String {
            match function_object.name.is_empty() {
                true => "<script>".into(),
                false => format!("<fn {}>", function_object.name),
            }
        }

        match self {
            Object::String(rc) => write!(f, "{}", rc.data.borrow()),
            Object::Function(rc) => write!(f, "{}", format_function_object(rc)),
            Object::NativeFunction(rc) => write!(f, "<native fn {}>", rc.name),
            Object::Closure(rc) => write!(f, "{}", format_function_object(&rc.function)),
            Object::Upvalue(_rc) => write!(f, "upvalue",),
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
    pub upvalue_count: u8,
    pub chunk: Chunk,
}

impl FunctionObject {
    pub fn new(name: &str, kind: FunctionKind) -> Self {
        Self {
            common: ObjectCommon::default(),
            chunk: Chunk::new(),
            name: name.into(),
            kind,
            upvalue_count: 0,
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

#[derive(Debug)]
pub struct ClosureObject {
    pub common: ObjectCommon,
    pub function: Rc<FunctionObject>,
    pub upvalues: [Rc<RefCell<UpvalueObject>>; u8::MAX as usize],
    pub upvalue_count: u8,
}

impl ClosureObject {
    pub fn new(function: Rc<FunctionObject>) -> Self {
        let upvalues = std::array::from_fn(|_| Rc::new(RefCell::new(UpvalueObject::default())));
        Self {
            common: ObjectCommon::default(),
            upvalue_count: function.upvalue_count,
            function,
            upvalues,
        }
    }
}

#[derive(Debug)]
pub struct UpvalueObject {
    pub common: ObjectCommon,
    pub value_slot: usize,
    pub is_closed: bool, // no clever pointer redirection in closed_upvalues since we used value_slot as usize, not some *Value
    pub closed: Value,
}

impl Default for UpvalueObject {
    fn default() -> Self {
        Self {
            common: ObjectCommon::default(),
            value_slot: 0,
            is_closed: false,
            closed: Value::Uninit,
        }
    }
}

impl UpvalueObject {
    pub fn new(value_slot: usize) -> Self {
        Self {
            common: ObjectCommon::default(),
            value_slot,
            is_closed: false,
            closed: Value::Uninit,
        }
    }
}
