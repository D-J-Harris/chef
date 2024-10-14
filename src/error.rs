use thiserror::Error;

pub type InterpretResult<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Compile error.")] // TODO: remove
    CompileError,
    #[error("Index out of bounds.")]
    OutOfBounds,
    #[error("Attempted to read from uninitialized stack slot.")]
    UninitializedStackValue,
    #[error("Attempted to read from uninitialized constant slot.")]
    UninitializedConstantValue,
    #[error("Attempted to read from uninitialized upvalue slot.")]
    UninitializedUpvalue,
    #[error("Stack overflow.")]
    StackOverflow,
    #[error("Only instances have properties.")]
    InstanceGetProperty,
    #[error("Only instances have fields.")]
    InstanceSetProperty,
    #[error("Call to closure does not have an associated function.")]
    ClosureGetFunction,
    #[error("Call to instance does not have an associated class.")]
    InstanceGetClass,
    #[error("Call to bound method does not have a closure.")]
    BoundMethodGetClosure,
    #[error("Expected {0} arguments but got {1}.")]
    FunctionArity(u8, u8),
    #[error("Can only call functions and classes.")]
    InvalidCallee,
    #[error("Undefined variable '{0}'.")]
    UndefinedVariable(String),
    #[error("Undefined property '{0}'.")]
    UndefinedProperty(String),
    #[error("Operand must be a number.")]
    ValueNegationOperation,
    #[error("Operands must be numbers.")]
    ValueNumberOnlyOperation,
    #[error("Operands must both be numbers or both be strings.")]
    ValueAddOperation,
    #[error("Operand must be a number, boolean or nil.")]
    ValueFalsinessOperation,
    #[error("No string name initialized.")]
    ConstantStringNotFound,
    #[error("No function name initialized.")]
    ConstantFunctionNotFound,
    #[error("No closure name initialized.")]
    ConstantClosureNotFound,
    #[error("No class name initialized.")]
    ConstantClassNotFound,
    #[error("No instance to bind method to.")]
    BindMethodReceiver,
    #[error("Invalid closure opcodes")]
    ClosureOpcode, // TODO: can be removed with more trust in code?
    #[error("Generic error while transitioning.")] // TODO: remove once done
    GenericRuntimeError,
}
