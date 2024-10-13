use thiserror::Error;

pub type Result<T> = std::result::Result<T, ChefError>;

#[derive(Debug, Error)]
pub enum ChefError {
    #[error("Compile error.")]
    CompileError,
    #[error("Runtime error.")]
    RuntimeError,
}
