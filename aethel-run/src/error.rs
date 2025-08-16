use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Nix syscall failed: {0}")]
    Nix(#[from] nix::Error),

    #[error("Failed to create C-style string for command: {0}")]
    InvalidCString(#[from] std::ffi::NulError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
} 