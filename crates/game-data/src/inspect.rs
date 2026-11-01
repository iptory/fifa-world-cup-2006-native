use std::path::Path;

use anyhow::Context;

use crate::{DiscInfo, SUPPORTED_MEDIA_ID, SUPPORTED_TITLE_ID, SUPPORTED_VERSION, xex};
use sha2::Digest as _;

pub(crate) fn inspect_inner(path: &Path) -> anyhow::Result<DiscInfo> {
    let mut image = crate::image::open(path)?;
    let root = image.read_volume_root()?;
    let entries = crate::tree::collect(&mut image, root)?;
    let exe = find(&entries, crate::EXECUTABLE_NAME)?;
    if exe.is_dir {
        anyhow::bail!("{} is a directory", crate::EXECUTABLE_NAME);
    }
    let bytes = image.read_bytes(exe.data_offset, exe.data_len, 64 * 1024 * 1024)?;
    let (title, media, version) = xex::execution_ids(&bytes)?;
    if title != SUPPORTED_TITLE_ID || media != SUPPORTED_MEDIA_ID || version != SUPPORTED_VERSION {
        anyhow::bail!(
            "unsupported disc revision: title {title:#010x} media {media:#010x} version {version:#010x}"
        );
    }
    let digest = sha2::Sha256::digest(&bytes);
    let executable_sha256 = format!("{digest:x}");
    if executable_sha256 != crate::SUPPORTED_EXECUTABLE_SHA256 {
        anyhow::bail!("unsupported executable revision {executable_sha256}");
    }
    let version_entry = find(&entries, crate::VERSION_FILE_NAME)?;
    if version_entry.is_dir {
        anyhow::bail!("{} is a directory", crate::VERSION_FILE_NAME);
    }
    let version_bytes = image.read_bytes(version_entry.data_offset, version_entry.data_len, 64)?;
    let text = core::str::from_utf8(&version_bytes)
        .map_err(|_| anyhow::anyhow!("version.txt is not UTF-8"))?;
    if text.trim() != crate::SUPPORTED_VERSION_TXT {
        anyhow::bail!("unsupported version.txt {:?}", text.trim());
    }
    Ok(DiscInfo {
        title_id: title,
        media_id: media,
        version,
        executable_sha256,
    })
}

fn find<'a>(
    entries: &'a [crate::tree::Entry],
    name: &str,
) -> anyhow::Result<&'a crate::tree::Entry> {
    entries
        .iter()
        .find(|e| e.rel.eq_ignore_ascii_case(name))
        .with_context(|| format!("ISO missing {name}"))
}
