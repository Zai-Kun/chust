use thiserror::Error;

/// Core error type representing all failure modes within `chust-lib`.
#[derive(Error, Debug)]
pub enum ChustError {
    #[error("Failed to load or initialize ONNX model: {0}")]
    ModelLoad(String),

    #[error("Model inference execution failed: {0}")]
    InferenceFailed(String),

    #[error("Failed to extract tensor `{name}`: {reason}")]
    TensorExtraction { name: &'static str, reason: String },

    #[error("Output tensor shape or dimension mismatch: {0}")]
    TensorShapeMismatch(String),

    #[error("No chessboard detected in the image meeting confidence criteria")]
    BoardNotFound,

    #[error("Invalid algebraic chess notation `{0}` (expected format: 'e4', 'a1')")]
    InvalidNotation(String),

    #[error("Failed to load or parse font data: {0}")]
    FontLoad(String),
}

/// Convenience alias for `Result<T, ChustError>`.
pub type Result<T> = std::result::Result<T, ChustError>;
