use crate::process::{AethelProcess, Process};
use aethel_common::error::{AethelError, Result};
use std::ffi::CString;
use std::os::unix::io::RawFd;
use nix::unistd;

pub struct Container<P: Process> {
    id: String,
    process: P,
}

impl<P: Process> Container<P> {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn wait(&self) -> Result<()> {
        self.process.wait()
    }
}

use std::path::{Path, PathBuf};

pub struct ContainerBuilder {
    id: String,
    command: CString,
    args: Vec<CString>,
    rootfs: PathBuf,
}

impl ContainerBuilder {
    pub fn new(id: &str, command: &str) -> Result<Self> {
        let command = CString::new(command)
            .map_err(|_| AethelError::ContainerSetup("Command contains interior NUL byte".to_string()))?;
        Ok(ContainerBuilder {
            id: id.to_string(),
            command,
            args: vec![],
            rootfs: PathBuf::from("/"),
        })
    }

    pub fn with_rootfs(mut self, path: &Path) -> Self {
        self.rootfs = path.to_path_buf();
        self
    }

    pub fn args(mut self, args: &[&str]) -> Result<Self> {
        let mut parsed = Vec::with_capacity(args.len());
        for arg in args {
            let c = CString::new(*arg).map_err(|_| {
                AethelError::ContainerSetup("Argument contains interior NUL byte".to_string())
            })?;
            parsed.push(c);
        }
        self.args = parsed;
        Ok(self)
    }

    pub fn build(self) -> Result<(isize, RawFd)> {
        let (pipe_read, pipe_write) = unistd::pipe()?;
        let mut stack = [0; 1024 * 1024];
        let child_pid = AethelProcess::new(move || {
            if let Err(e) = {
                if let Err(err) = unistd::dup2(pipe_write, 1) { return Err(err); }
                if let Err(err) = unistd::dup2(pipe_write, 2) { return Err(err); }
                if let Err(err) = unistd::close(pipe_read) { return Err(err); }
                if let Err(err) = unistd::close(pipe_write) { return Err(err); }
                crate::namespaces::pivot_root(&self.rootfs)
            } {
                eprintln!("container setup failed: {}", e);
                return -1;
            }
            println!("Inside container");
            0
        }, &mut stack)?;

        unistd::close(pipe_write)?;

        Ok((child_pid, pipe_read))
    }
}
