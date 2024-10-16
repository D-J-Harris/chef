use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    chunk::Chunk,
    common::UPVALUES_MAX_COUNT,
    value::{FieldValue, Value},
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
    pub upvalue_count: usize,
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
    pub upvalue_count: usize,
    pub function: Rc<FunctionObject>,
    pub upvalues: [Option<Rc<RefCell<UpvalueObject>>>; UPVALUES_MAX_COUNT],
}

const UPVALUE_DEFAULT: Option<Rc<RefCell<UpvalueObject>>> = None;
impl ClosureObject {
    pub fn new(upvalue_count: usize, function: Rc<FunctionObject>) -> Self {
        Self {
            upvalue_count,
            function,
            upvalues: [UPVALUE_DEFAULT; UPVALUES_MAX_COUNT],
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
    pub class: Rc<RefCell<ClassObject>>,
    pub fields: HashMap<String, FieldValue>,
}

impl InstanceObject {
    pub fn new(class: Rc<RefCell<ClassObject>>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoundMethod {
    pub receiver: Rc<RefCell<InstanceObject>>,
    pub closure: Rc<ClosureObject>,
}

impl BoundMethod {
    pub fn new(receiver: Rc<RefCell<InstanceObject>>, closure: Rc<ClosureObject>) -> Self {
        Self { receiver, closure }
    }
}

impl PartialEq for BoundMethod {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.receiver, &other.receiver) && Rc::ptr_eq(&self.closure, &other.closure)
    }
}

#[cfg(feature = "debug_trace_gc")]
mod debug {
    use crate::common::print_function;

    use super::{
        BoundMethod, ClassObject, ClosureObject, FunctionObject, InstanceObject, UpvalueObject,
    };

    impl Drop for FunctionObject {
        fn drop(&mut self) {
            println!("Dropped function {}", print_function(&self.name))
        }
    }

    impl Drop for ClosureObject {
        fn drop(&mut self) {
            println!("Dropped closure {}", print_function(&self.function.name))
        }
    }

    impl Drop for UpvalueObject {
        fn drop(&mut self) {
            println!("Dropped upvalue {:?}", self)
        }
    }

    impl Drop for ClassObject {
        fn drop(&mut self) {
            println!("Dropped class {}", self.name)
        }
    }

    impl Drop for InstanceObject {
        fn drop(&mut self) {
            println!("Dropped class instance {}", self.class.borrow().name)
        }
    }

    impl Drop for BoundMethod {
        fn drop(&mut self) {
            println!(
                "Dropped bound method {} on receiver instance {}",
                print_function(&self.closure.function.name),
                self.receiver.borrow().class.borrow().name
            )
        }
    }
}
