use thiserror::Error;

pub type InterpretResult<T> = std::result::Result<T, RuntimeError>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Compiler")] // TODO: move out
    Compile,
    #[error("Index out of bounds.")]
    OutOfBounds,
    #[error("Attempted to read from uninitialized stack slot.")]
    UninitializedStackValue,
    #[error("Attempted to read from uninitialized constant slot.")]
    UninitializedConstantValue,
    #[error("Attempted to read from uninitialized constant slot.")]
    UninitializedUpvalue,
    #[error("Stack overflow.")]
    StackOverflow,
    #[error("Only instances have properties.")]
    InstanceGetProperty,
    #[error("Only instances have fields.")]
    InstanceSetProperty,
    #[error("Only instances have methods.")]
    InstanceInvoke,
    #[error("Can only call functions and classes.")]
    InvalidCallee,
    #[error("Expected {0} arguments but got {1}.")]
    FunctionArity(u8, u8),
    #[error("Expected 0 arguments but got {0}.")]
    ClassArguments(u8),
    #[error("Undefined variable '{0}'.")]
    UndefinedVariable(String), // TODO: better impl for error that takes Gc<String>
    #[error("Undefined property '{0}'.")]
    UndefinedProperty(String),
    #[error("Operand must be a number.")]
    ValueNegationOperation,
    #[error("Operands must be numbers.")]
    ValueNumberOnlyOperation,
    #[error("Operands must be two numbers or two strings.")]
    ValueAddOperation,
    #[error("No string name initialized.")]
    ConstantStringNotFound,
    #[error("No function name initialized.")]
    ConstantFunctionNotFound,
    #[error("No closure name initialized.")]
    ConstantClosureNotFound,
    #[error("No class name initialized.")]
    ConstantClassNotFound,
    #[error("Superclass must be a class.")]
    ConstantSuperclassNotFound,
    #[error("Invalid field reference.")]
    InstanceReferenceInvalid,
    #[error("Value on top of stack is not an instance")]
    NoInstanceOnStack,
    #[error("Invalid closure opcodes")]
    ClosureOpcode,
}
