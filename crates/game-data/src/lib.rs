use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod image;
mod import;
mod inspect;
mod manifest;
mod sanitize;
mod tree;
mod xex;

pub const SUPPORTED_EXECUTABLE_SHA256: &str =
    "cb184f76112c3e60ec658cc1398779c3b9e09f3007f4ec2b6482bd22b66929b6";
pub const SUPPORTED_TITLE_ID: u32 = 0x4541_07E6;
pub const SUPPORTED_MEDIA_ID: u32 = 0x4C1B_827B;
pub const SUPPORTED_VERSION: u32 = 8;
pub const SUPPORTED_VERSION_TXT: &str = "83769PAL_En";

pub(crate) const EXECUTABLE_NAME: &str = "default.xex";
pub(crate) const VERSION_FILE_NAME: &str = "version.txt";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscInfo {
    pub title_id: u32,
    pub media_id: u32,
    pub version: u32,
    pub executable_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedGame {
    pub root: PathBuf,
    pub disc: DiscInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportProgress {
    pub current_file: PathBuf,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

/// # Errors
/// Rejects unreadable, malformed or unsupported disc images without extracting them.
pub fn inspect_iso(path: &Path) -> anyhow::Result<DiscInfo> {
    inspect::inspect_inner(path)
}

/// # Errors
/// Rejects unsupported images, unsafe paths or I/O failures; reuses valid prepared data.
pub fn import_iso(
    source: &Path,
    destination: &Path,
    progress: impl FnMut(ImportProgress),
) -> anyhow::Result<PreparedGame> {
    import::import_inner(source, destination, progress)
}

/// # Errors
/// Unreadable or malformed manifests/files fail; missing or incompatible data returns `None`.
pub fn prepared_game(root: &Path) -> anyhow::Result<Option<PreparedGame>> {
    manifest::prepared_inner(root)
}
