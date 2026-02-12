use aethel_common::error::{AethelError, Result};
use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, pivot_root, sethostname};
use std::path::Path;

pub fn set_hostname(hostname: &str) -> Result<()> {
    sethostname(hostname).map_err(|e| AethelError::Namespace(format!("Failed to set hostname: {}", e)))?;
    Ok(())
}

pub fn pivot_root(new_root: &Path) -> Result<()> {
    mount(
        Some(new_root),
        new_root,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    let old_root_name = "old_root";
    let old_root = new_root.join(old_root_name);
    nix::unistd::mkdir(&old_root, nix::sys::stat::Mode::S_IRWXU)?;
    pivot_root(new_root, &old_root)?;
    chdir("/")?;

    mount(
        Some("proc"),
        &Path::new("/proc"),
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    )?;

    nix::unistd::umount2(&old_root, nix::mount::MntFlags::MNT_DETACH)?;
    nix::unistd::rmdir(&old_root)?;

    Ok(())
}