//! Error types for gitcore

use thiserror::Error;

/// Result type for gitcore operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for gitcore
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid object format
    #[error("Invalid object format: {0}")]
    InvalidObjectFormat(String),

    /// Object not found
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    /// Invalid SHA256 hash
    #[error("Invalid SHA256 hash: {0}")]
    InvalidHash(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Merge conflict
    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    /// Invalid tree entry
    #[error("Invalid tree entry: {0}")]
    InvalidTreeEntry(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Compression error
    #[error("Compression error: {0}")]
    CompressionError(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}
