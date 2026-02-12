use aethel_common::error::{AethelError, Result};
use nix::sched::{clone, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use std::ffi::CString;

pub trait Process {
    fn id(&self) -> Pid;
    fn wait(&self) -> Result<()>;
}

pub struct AethelProcess {
    pid: Pid,
}

impl AethelProcess {
    pub fn new<F>(f: F, stack: &mut [u8]) -> Result<Self>
    where
        F: FnOnce() -> isize,
    {
        let flags = CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS;
        let child_pid = syscall!(clone(Box::new(f), stack, flags, Some(nix::sys::signal::Signal::SIGCHLD as i32)))?;

        Ok(AethelProcess { pid: child_pid })
    }
}

impl Process for AethelProcess {
    fn id(&self) -> Pid {
        self.pid
    }

    fn wait(&self) -> Result<()> {
        waitpid(self.pid, None).map_err(|e| AethelError::Process(format!("Failed to wait for process: {}", e)))?;
        Ok(())
    }
}