use std::{collections::HashMap, ops::Deref};

use gc_arena::{lock::RefLock, Collect, Gc};

use crate::{chunk::Chunk, common::UPVALUES_MAX_COUNT, value::Value};

pub type NativeFunction<'gc> = fn(arg_count: u8, ip: usize) -> Value<'gc>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionKind {
    Script,
    Function,
    Method,
    Initializer,
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct FunctionObject<'gc> {
    pub arity: u8,
    pub chunk: Chunk<'gc>,
    pub name: String,
    #[collect(require_static)]
    pub kind: FunctionKind,
    pub upvalue_count: usize,
}

impl<'gc> FunctionObject<'gc> {
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

impl<'gc> Deref for FunctionObject<'gc> {
    type Target = Chunk<'gc>;
    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct ClosureObject<'gc> {
    pub upvalue_count: usize,
    pub function: Gc<'gc, FunctionObject<'gc>>,
    pub upvalues: Vec<Gc<'gc, RefLock<UpvalueObject<'gc>>>>,
}

impl<'gc> ClosureObject<'gc> {
    pub fn new(upvalue_count: usize, function: Gc<'gc, FunctionObject<'gc>>) -> Self {
        Self {
            upvalue_count,
            function,
            upvalues: Vec::with_capacity(UPVALUES_MAX_COUNT),
        }
    }
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub enum UpvalueObject<'gc> {
    Open(usize),
    Closed(Value<'gc>),
}

impl<'gc> UpvalueObject<'gc> {
    pub fn new(stack_index: usize) -> Self {
        Self::Open(stack_index)
    }
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct ClassObject<'gc> {
    pub name: String,
    pub methods: HashMap<Gc<'gc, String>, Gc<'gc, ClosureObject<'gc>>>,
}

impl<'gc> ClassObject<'gc> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            methods: HashMap::new(),
        }
    }

    pub fn add_method(&mut self, name: Gc<'gc, String>, value: Gc<'gc, ClosureObject<'gc>>) {
        self.methods.insert(name, value);
    }
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct InstanceObject<'gc> {
    pub class: Gc<'gc, RefLock<ClassObject<'gc>>>,
    pub fields: HashMap<Gc<'gc, String>, Value<'gc>>,
}

impl<'gc> InstanceObject<'gc> {
    pub fn new(class: Gc<'gc, RefLock<ClassObject<'gc>>>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }
}

#[derive(Debug, Copy, Clone, Collect)]
#[collect(no_drop)]
pub struct BoundMethod<'gc> {
    pub receiver: Gc<'gc, RefLock<InstanceObject<'gc>>>,
    pub closure: Gc<'gc, ClosureObject<'gc>>,
}

impl<'gc> BoundMethod<'gc> {
    pub fn new(
        receiver: Gc<'gc, RefLock<InstanceObject<'gc>>>,
        closure: Gc<'gc, ClosureObject<'gc>>,
    ) -> Self {
        Self { receiver, closure }
    }
}

impl<'gc> PartialEq for BoundMethod<'gc> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.receiver, &other.receiver) && std::ptr::eq(&self.closure, &other.closure)
    }
}
