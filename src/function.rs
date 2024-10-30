use crate::chunk::Code;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionKind {
    Script,
    Function,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub arity: u8,
    pub chunk: Code,
    pub kind: FunctionKind,
    pub upvalues: Vec<usize>,
    pub upvalues_count: usize,
}

impl Function {
    pub fn new(name: String, kind: FunctionKind) -> Self {
        let chunk = Code::new();
        Self {
            chunk,
            name,
            kind,
            arity: 0,
            upvalues: Vec::new(),
            upvalues_count: 0,
        }
    }
}
