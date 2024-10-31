use thiserror::Error;

pub type InterpretResult<T> = std::result::Result<T, ChefError>;

#[derive(Debug, Error)]
pub enum ChefError {
    #[error("Could not compile.")]
    Compile,
    #[error("Index out of bounds.")]
    OutOfBounds,
    #[error("Stack overflow.")]
    StackOverflow,
    #[error("Can only call functions.")]
    InvalidCallee,
    #[error("Expected {0} arguments but got {1}.")]
    FunctionArity(u8, u8),
    #[error("Operand must be a number.")]
    ValueNegationOperation,
    #[error("Operands must be numbers.")]
    ValueNumberOnlyOperation,
    #[error("Operands must be two numbers or two strings.")]
    ValueAddOperation,
}
