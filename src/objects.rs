use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    chunk::Chunk,
    common::U8_MAX,
    value::{Value, WeakValue},
};

pub type NativeFunction = fn(arg_count: u8, ip: usize) -> Value;

#[derive(Debug)]
pub struct NativeFunctionObject {
    pub name: String,
    pub function: NativeFunction,
}

impl NativeFunctionObject {
    pub fn new(name: &str, function: NativeFunction) -> Self {
        Self {
            name: name.into(),
            function,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionKind {
    Script,
    Function,
    Method,
    Initializer,
}

#[derive(Debug)]
pub struct FunctionObject {
    pub arity: u8,
    pub chunk: Chunk,
    pub name: String,
    pub kind: FunctionKind,
    pub upvalue_count: u8,
}

impl FunctionObject {
    pub fn new(name: String, kind: FunctionKind) -> Self {
        let chunk = Chunk::new();
        Self {
            chunk,
            name,
            kind,
            arity: 0,
            upvalue_count: 0,
        }
    }
}

#[derive(Debug)]
pub struct ClosureObject {
    pub upvalue_count: u8,
    pub function_name: String,
    pub function: Weak<FunctionObject>,
    pub upvalues: [Option<Rc<RefCell<UpvalueObject>>>; U8_MAX],
}

const UPVALUE_DEFAULT: Option<Rc<RefCell<UpvalueObject>>> = None;
impl ClosureObject {
    pub fn new(function_name: &str, upvalue_count: u8, function: Weak<FunctionObject>) -> Self {
        Self {
            upvalue_count,
            function_name: function_name.into(),
            function,
            upvalues: [UPVALUE_DEFAULT; U8_MAX],
        }
    }
}

#[derive(Debug)]
pub enum UpvalueObject {
    Open(usize),
    Closed(Value),
}

impl UpvalueObject {
    pub fn new(value_slot: usize) -> Self {
        Self::Open(value_slot)
    }
}

#[derive(Debug)]
pub struct ClassObject {
    pub name: String,
    pub methods: HashMap<String, Rc<ClosureObject>>,
}

impl ClassObject {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            methods: HashMap::new(),
        }
    }

    pub fn add_method(&mut self, name: String, value: Rc<ClosureObject>) {
        self.methods.insert(name, value);
    }
}

#[derive(Debug)]
pub struct InstanceObject {
    pub class: Weak<RefCell<ClassObject>>,
    pub fields: HashMap<String, WeakValue>,
    pub bound_methods: Vec<Rc<BoundMethodObject>>,
}

impl InstanceObject {
    pub fn new(class: Weak<RefCell<ClassObject>>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
            bound_methods: Vec::new(),
        }
    }

    pub fn add_bound_method(&mut self, bound_method: Rc<BoundMethodObject>) {
        self.bound_methods.push(bound_method);
    }
}

#[derive(Debug)]
pub struct BoundMethodObject {
    pub receiver: Weak<RefCell<InstanceObject>>,
    pub closure: Weak<ClosureObject>,
}

impl BoundMethodObject {
    pub fn new(receiver: Weak<RefCell<InstanceObject>>, closure: Weak<ClosureObject>) -> Self {
        Self { receiver, closure }
    }
}

// TODO: better messages here
#[cfg(feature = "debug_trace_gc")]
mod debug {
    use super::{
        BoundMethodObject, ClassObject, ClosureObject, FunctionObject, InstanceObject,
        UpvalueObject,
    };

    impl Drop for FunctionObject {
        fn drop(&mut self) {
            let name = match self.name.is_empty() {
                true => "<script>",
                false => &self.name,
            };
            println!("dropped function {}", name)
        }
    }

    impl Drop for ClosureObject {
        fn drop(&mut self) {
            let name = match self.function_name.is_empty() {
                true => "<script>",
                false => &self.function_name,
            };
            println!("dropped closure {}", name)
        }
    }

    impl Drop for UpvalueObject {
        fn drop(&mut self) {
            println!("dropped upvalue {:?}", self)
        }
    }

    impl Drop for ClassObject {
        fn drop(&mut self) {
            println!("dropped class {}", self.name)
        }
    }

    impl Drop for InstanceObject {
        fn drop(&mut self) {
            println!("dropped class instance {:?}", self)
        }
    }

    impl Drop for BoundMethodObject {
        fn drop(&mut self) {
            println!("dropped bound method {:?}", self)
        }
    }
}
