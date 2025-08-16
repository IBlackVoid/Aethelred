use crate::error::Error;
use crate::process::Result;
use nix::mount::{mount, MsFlags, umount2, MntFlags};
use nix::unistd::{chdir, sethostname, mkdir};
use std::path::Path;
use std::fs;

pub fn set_hostname(hostname: &str) -> Result<()> {
    sethostname(hostname)?;
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
    mkdir(&old_root, nix::sys::stat::Mode::S_IRWXU)?;
    nix::unistd::pivot_root(new_root, &old_root)?;
    chdir("/")?;

    mount(
        Some("proc"),
        Path::new("/proc"),
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    )?;

    umount2(&old_root, MntFlags::MNT_DETACH)?;
    fs::remove_dir(&old_root).map_err(Error::Io)?;

    Ok(())
}