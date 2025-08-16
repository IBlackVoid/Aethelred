use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Filesystem error: {0}")]
    Filesystem(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to deserialize JSON from manifest: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Failed to walk directory: {0}")]
    Walkdir(#[from] walkdir::Error),
} 