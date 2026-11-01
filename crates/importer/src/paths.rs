use std::path::{Path, PathBuf};

use anyhow::Context as _;

/// # Errors
/// Fails if the executable's path or parent cannot be determined.
pub fn exe_dir() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe().context("locating importer executable")?;
    exe.parent()
        .map(Path::to_path_buf)
        .context("importer executable has no parent directory")
}

/// # Errors
/// Fails if the setup directory cannot be determined; data belongs in its `prepared/game`.
pub fn default_destination() -> anyhow::Result<PathBuf> {
    Ok(exe_dir()?.join("prepared").join("game"))
}

/// # Errors
/// Fails if the current directory is unavailable.
pub fn absolute(path: &Path) -> anyhow::Result<PathBuf> {
    std::path::absolute(path).context("resolving absolute path")
}
