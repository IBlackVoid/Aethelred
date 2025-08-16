use crate::process::{AethelProcess, Process, Result};
use std::ffi::CString;
use std::os::unix::io::{IntoRawFd, RawFd};
use nix::unistd;
use std::path::{Path, PathBuf};

pub struct Container<P: Process> {
    id: String,
    process: P,
    log_fd: RawFd,
}

impl<P: Process> Container<P> {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn pid(&self) -> nix::unistd::Pid {
        self.process.id()
    }

    pub fn wait(&self) -> Result<()> {
        self.process.wait()
    }

    /// Returns the read-end file descriptor of the pipe that captures the
    /// container's stdout/stderr. This can be used by the daemon to forward
    /// log lines over gRPC.
    pub fn log_fd(&self) -> RawFd {
        self.log_fd
    }
}

pub struct ContainerBuilder {
    id: String,
    command: CString,
    args: Vec<CString>,
    rootfs: PathBuf,
}

impl ContainerBuilder {
    pub fn new(id: &str, command: &str) -> Result<Self> {
        Ok(ContainerBuilder {
            id: id.to_string(),
            command: CString::new(command)?,
            args: vec![],
            rootfs: PathBuf::from("/"),
        })
    }

    pub fn with_rootfs(mut self, path: &Path) -> Self {
        self.rootfs = path.to_path_buf();
        self
    }

    pub fn args(mut self, args: &[&str]) -> Result<Self> {
        self.args = args.iter().map(|s| CString::new(*s)).collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(self)
    }

    pub fn build(self) -> Result<Container<AethelProcess>> {
        let rootfs_path = self.rootfs.clone();
        let command_cstr = self.command.clone();
        let args_cstr = self.args.clone();
        let (pipe_read, pipe_write) = {
            let (read_fd, write_fd) = unistd::pipe()?;
            (read_fd.into_raw_fd(), write_fd.into_raw_fd())
        };
        let mut stack = [0; 1024 * 1024];
        let child_pid = AethelProcess::new(
            move || {
                // Duplicate stdout/stderr to the write end of our pipe so the
                // daemon can capture everything the container prints.
                unistd::dup2(pipe_write, 1).ok();
                unistd::dup2(pipe_write, 2).ok();
                // Close our copies of the pipe fds; the read end stays open in the
                // parent, the write end lives on as stdout/stderr.
                let _ = unistd::close(pipe_read);
                let _ = unistd::close(pipe_write);

                // Perform pivot_root only if the new root looks usable (has /bin/sh).
                if rootfs_path.join("bin/sh").exists() {
                    if let Err(e) = crate::namespaces::pivot_root(&rootfs_path) {
                        eprintln!("pivot_root failed: {e:?}; continuing inside host rootfs");
                    }
                } else {
                    eprintln!("rootfs {:?} lacks /bin/sh; skipping pivot_root", rootfs_path);
                }

                // Build argv for execvp – first element must be the command itself.
                use std::ffi::CStr;
                let mut argv: Vec<&CStr> = Vec::with_capacity(1 + args_cstr.len());
                argv.push(command_cstr.as_c_str());
                for arg in &args_cstr {
                    argv.push(arg.as_c_str());
                }

                // Replace the init process with the requested command.
                if let Err(e) = nix::unistd::execvp(command_cstr.as_c_str(), &argv) {
                    eprintln!("execvp failed inside container: {e}");
                    return -1; // Make the process exit with error so the daemon can notice.
                }
                0
            },
            &mut stack,
        )?;

        // Parent: close the write end; keep the read end for log streaming.
        unistd::close(pipe_write)?;

        Ok(Container {
            id: self.id,
            process: child_pid,
            log_fd: pipe_read,
        })
    }
}