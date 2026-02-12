use aethel_common::error::{AethelError, Result};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tar::Archive;

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
    let index_path = image_path.join("index.json");
    let index_file = File::open(index_path)?;
    let index: OciIndex = serde_json::from_reader(index_file)
        .map_err(|e| AethelError::Filesystem(format!("Failed to parse index.json: {}", e)))?;

    let manifest_ref = index.manifests.get(0).ok_or_else(|| {
        AethelError::Filesystem("No manifests found in index.json".to_string())
    })?;

    let manifest_path = image_path
        .join("blobs")
        .join("sha256")
        .join(&manifest_ref.digest[7..]);
    let manifest_file = File::open(manifest_path)?;
    let manifest: OciManifest = serde_json::from_reader(manifest_file)
        .map_err(|e| AethelError::Filesystem(format!("Failed to parse manifest: {}", e)))?;

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