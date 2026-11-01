use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{DiscInfo, PreparedGame};

pub(crate) const MANIFEST_NAME: &str = "fifa-game-data.json";
pub(crate) const VERSION_TXT_MAX: u64 = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) title_id: u32,
    pub(crate) media_id: u32,
    pub(crate) version: u32,
    pub(crate) executable_sha256: String,
    pub(crate) files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManifestFile {
    pub(crate) path: String,
    pub(crate) size: u64,
}

pub(crate) fn is_system_update(rel: &str) -> bool {
    rel.split('/')
        .next()
        .is_some_and(|top| top.eq_ignore_ascii_case("$SystemUpdate"))
}

pub(crate) fn prepared_inner(root: &Path) -> anyhow::Result<Option<PreparedGame>> {
    let manifest_path = root.join(MANIFEST_NAME);
    let manifest_bytes = match std::fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err).with_context(|| format!("read {}", manifest_path.display()));
        }
    };
    if manifest_bytes.len() > 16 * 1024 * 1024 {
        anyhow::bail!("prepared manifest too large");
    }
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
        .with_context(|| format!("parse {}", manifest_path.display()))?;
    if manifest.title_id != crate::SUPPORTED_TITLE_ID
        || manifest.media_id != crate::SUPPORTED_MEDIA_ID
        || manifest.version != crate::SUPPORTED_VERSION
        || manifest.executable_sha256 != crate::SUPPORTED_EXECUTABLE_SHA256
    {
        return Ok(None);
    }
    check_files(root, &manifest)
}

fn check_files(root: &Path, manifest: &Manifest) -> anyhow::Result<Option<PreparedGame>> {
    if manifest.files.iter().any(|f| is_system_update(&f.path)) {
        return Ok(None);
    }
    for file in &manifest.files {
        let target = crate::sanitize::join_inside(root, &file.path)?;
        assert_inside(root, &target)?;
        let meta = match std::fs::metadata(&target) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).with_context(|| format!("stat {}", target.display())),
        };
        if !meta.is_file() || meta.len() != file.size {
            return Ok(None);
        }
    }
    let exe = crate::sanitize::join_inside(root, crate::EXECUTABLE_NAME)?;
    if hash_file(&exe)? != manifest.executable_sha256 {
        return Ok(None);
    }
    Ok(Some(PreparedGame {
        root: root.to_path_buf(),
        disc: DiscInfo {
            title_id: manifest.title_id,
            media_id: manifest.media_id,
            version: manifest.version,
            executable_sha256: manifest.executable_sha256.clone(),
        },
    }))
}

pub(crate) fn prepare_staging(destination: &Path) -> anyhow::Result<tempfile::TempDir> {
    let parent = destination.parent().with_context(|| {
        format!(
            "destination {} has no parent directory",
            destination.display()
        )
    })?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create destination parent {}", parent.display()))?;
    tempfile::Builder::new()
        .prefix(".fifa-import-")
        .tempdir_in(parent)
        .with_context(|| format!("create staging in {}", parent.display()))
}

pub(crate) fn publish_staging(staging: &Path, destination: &Path) -> anyhow::Result<()> {
    if !destination.exists() {
        return std::fs::rename(staging, destination)
            .with_context(|| format!("publish {}", destination.display()));
    }
    let existing: Manifest = serde_json::from_slice(
        &std::fs::read(destination.join(MANIFEST_NAME))
            .context("Refusing to replace a destination not owned by the importer")?,
    )
    .context("Existing destination has an invalid game-data manifest")?;
    anyhow::ensure!(
        existing.title_id == crate::SUPPORTED_TITLE_ID
            && existing.media_id == crate::SUPPORTED_MEDIA_ID
            && existing.version == crate::SUPPORTED_VERSION
            && existing.executable_sha256 == crate::SUPPORTED_EXECUTABLE_SHA256,
        "Refusing to replace a destination belonging to a different revision"
    );
    let parent = destination.parent().context("Destination has no parent")?;
    let backup = tempfile::Builder::new()
        .prefix(".fifa-backup-")
        .tempdir_in(parent)?;
    let previous = backup.path().join("game");
    std::fs::rename(destination, &previous).context("Move previous prepared data aside")?;
    if let Err(error) = std::fs::rename(staging, destination) {
        if let Err(restore) = std::fs::rename(&previous, destination) {
            let retained = backup.keep();
            anyhow::bail!(
                "Publish failed: {error}; restore failed: {restore}. Previous data retained at {}",
                retained.display()
            );
        }
        return Err(error).context("Publish prepared data; previous data restored");
    }
    Ok(())
}

pub(crate) fn assert_inside(root: &Path, target: &Path) -> anyhow::Result<()> {
    if target == root || target.starts_with(root) {
        Ok(())
    } else {
        anyhow::bail!("prepared path escapes root")
    }
}

pub(crate) fn verify_executable(path: &Path, disc: &DiscInfo) -> anyhow::Result<()> {
    let digest = hash_file(path)?;
    if digest != disc.executable_sha256 || digest != crate::SUPPORTED_EXECUTABLE_SHA256 {
        anyhow::bail!("prepared executable revision mismatch");
    }
    Ok(())
}

pub(crate) fn verify_version_txt(path: &Path) -> anyhow::Result<()> {
    let len = std::fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .len();
    if len > VERSION_TXT_MAX {
        anyhow::bail!("version.txt too large ({len})");
    }
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let text =
        core::str::from_utf8(&bytes).map_err(|_| anyhow::anyhow!("version.txt not UTF-8"))?;
    if text.trim() != crate::SUPPORTED_VERSION_TXT {
        anyhow::bail!("prepared version.txt mismatch {:?}", text.trim());
    }
    Ok(())
}

fn hash_file(path: &Path) -> anyhow::Result<String> {
    use sha2::Digest as _;
    let mut file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0_u8; 16384];
    loop {
        let read = std::io::Read::read(&mut file, &mut buf)
            .with_context(|| format!("hash {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn first_run_staging_is_owned_and_cleans_up_without_touching_neighbors() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let unrelated = directory.path().join("game.staging");
        std::fs::create_dir(&unrelated)?;
        std::fs::write(unrelated.join("keep"), b"user data")?;
        let destination = directory.path().join("prepared/game");
        let staging = super::prepare_staging(&destination)?;
        let staged_path = staging.path().to_path_buf();
        std::fs::write(staging.path().join("partial"), b"incomplete")?;
        drop(staging);
        assert!(!staged_path.exists());
        assert_eq!(std::fs::read(unrelated.join("keep"))?, b"user data");
        assert!(!destination.exists());
        Ok(())
    }
}
