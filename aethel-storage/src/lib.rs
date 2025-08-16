pub mod error;

use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tar::Archive;
use walkdir;
use crate::error::Error;

type Result<T> = std::result::Result<T, Error>;

#[derive(Deserialize)]
struct OciIndex {
    manifests: Vec<OciManifestRef>,
}

#[derive(Deserialize)]
struct OciManifestRef {
    digest: String,
}

#[derive(Deserialize)]
struct OciManifest {
    layers: Vec<OciLayer>,
}

#[derive(Deserialize)]
struct OciLayer {
    digest: String,
}

pub fn prepare_rootfs(image_name: &str) -> Result<PathBuf> {
    let image_path = Path::new("images").join(image_name);

    // Fast-path: if the image already contains a ready-to-use rootfs tree we
    // just copy/sync it into place and skip the OCI dance.  This is handy for
    // lightweight demo images committed directly into the repository.
    let prepop_rootfs = image_path.join("rootfs");
    if prepop_rootfs.exists() {
        let dest = Path::new("rootfs").join(image_name);
        fs::remove_dir_all(&dest).ok();
        fs::create_dir_all(&dest)?;
        // Recursively copy files.
        for entry_res in walkdir::WalkDir::new(&prepop_rootfs) {
            let entry = entry_res?;
            let rel = entry.path().strip_prefix(&prepop_rootfs).unwrap();
            let target = dest.join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() { fs::create_dir_all(parent)?; }
                fs::copy(entry.path(), &target)?;
            }
        }
        return Ok(dest);
    }

    let index_path = image_path.join("index.json");
    let index_file = File::open(index_path)?;
    let index: OciIndex = serde_json::from_reader(index_file)?;

    let manifest_ref = index.manifests.get(0).ok_or_else(|| {
        Error::Filesystem("No manifests found in index.json".to_string())
    })?;

    let manifest_path = image_path
        .join("blobs")
        .join("sha256")
        .join(&manifest_ref.digest[7..]);
    let manifest_file = File::open(manifest_path)?;
    let manifest: OciManifest = serde_json::from_reader(manifest_file)?;

    let rootfs_path = Path::new("rootfs").join(image_name);
    fs::create_dir_all(&rootfs_path)?;

    for layer in manifest.layers {
        let layer_path = image_path
            .join("blobs")
            .join("sha256")
            .join(&layer.digest[7..]);
        let tar_gz = File::open(layer_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&rootfs_path)?;
    }

    Ok(rootfs_path)
}