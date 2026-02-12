use std::fmt;
use std::io;
use nix;

#[derive(Debug)]
pub enum AethelError {
    Io(io::Error),
    Nix(nix::Error),
    ContainerSetup(String),
    Filesystem(String),
    Namespace(String),
    Cgroup(String),
    Process(String),
}

impl fmt::Display for AethelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AethelError::Io(e) => write!(f, "IO Error: {}", e),
            AethelError::Nix(e) => write!(f, "Nix Error: {}", e),
            AethelError::ContainerSetup(s) => write!(f, "Container Setup Error: {}", s),
            AethelError::Filesystem(s) => write!(f, "Filesystem Error: {}", s),
            AethelError::Namespace(s) => write!(f, "Namespace Error: {}", s),
            AethelError::Cgroup(s) => write!(f, "Cgroup Error: {}", s),
            AethelError::Process(s) => write!(f, "Process Error: {}", s),
        }
    }
}

impl std::error::Error for AethelError {}

impl From<io::Error> for AethelError {
    fn from(err: io::Error) -> AethelError {
        AethelError::Io(err)
    }
}

impl From<nix::Error> for AethelError {
    fn from(err: nix::Error) -> AethelError {
        AethelError::Nix(err)
    }
}

pub type Result<T> = std::result::Result<T, AethelError>;