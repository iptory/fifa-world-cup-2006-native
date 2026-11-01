use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::manifest::{Manifest, ManifestFile, is_system_update};
use crate::{DiscInfo, ImportProgress, PreparedGame};

pub(crate) fn import_inner(
    source: &Path,
    destination: &Path,
    mut progress: impl FnMut(ImportProgress),
) -> anyhow::Result<PreparedGame> {
    let reuse_candidate = crate::manifest::prepared_inner(destination)?;
    if let Some(reused) = reuse_candidate {
        return Ok(reused);
    }
    let disc = crate::inspect_iso(source)?;
    let mut image = crate::image::open(source)?;
    let root = image.read_volume_root()?;
    let entries = crate::tree::collect(&mut image, root)?;
    let total: u64 = entries
        .iter()
        .filter(|e| !e.is_dir && !is_system_update(&e.rel))
        .map(|e| e.data_len)
        .try_fold(0_u64, |acc, len| {
            acc.checked_add(len).context("total size overflow")
        })?;

    let staging_directory = crate::manifest::prepare_staging(destination)?;
    let staging = staging_directory.path();
    let mut files: Vec<ManifestFile> = Vec::new();
    let mut completed = 0_u64;
    progress(ImportProgress {
        current_file: PathBuf::from(""),
        completed_bytes: 0,
        total_bytes: total,
    });
    stream_all(
        &mut image,
        &entries,
        staging,
        total,
        &disc,
        &mut files,
        &mut completed,
        &mut progress,
    )?;
    crate::manifest::verify_executable(
        &crate::sanitize::join_inside(staging, crate::EXECUTABLE_NAME)?,
        &disc,
    )?;
    crate::manifest::verify_version_txt(&crate::sanitize::join_inside(
        staging,
        crate::VERSION_FILE_NAME,
    )?)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = Manifest {
        title_id: disc.title_id,
        media_id: disc.media_id,
        version: disc.version,
        executable_sha256: disc.executable_sha256.clone(),
        files,
    };
    let manifest_text =
        serde_json::to_string_pretty(&manifest).context("serialize prepared manifest")?;
    let manifest_path = staging.join(crate::manifest::MANIFEST_NAME);
    std::fs::write(&manifest_path, manifest_text)
        .with_context(|| format!("write {}", manifest_path.display()))?;
    crate::manifest::publish_staging(staging, destination)?;
    progress(ImportProgress {
        current_file: PathBuf::from(""),
        completed_bytes: total,
        total_bytes: total,
    });
    Ok(PreparedGame {
        root: destination.to_path_buf(),
        disc,
    })
}

#[allow(clippy::too_many_arguments)]
fn stream_all(
    image: &mut crate::image::Image,
    entries: &[crate::tree::Entry],
    staging: &Path,
    total: u64,
    disc: &DiscInfo,
    files: &mut Vec<ManifestFile>,
    done: &mut u64,
    progress: &mut impl FnMut(ImportProgress),
) -> anyhow::Result<()> {
    for entry in entries {
        if is_system_update(&entry.rel) {
            continue;
        }
        let target = crate::sanitize::join_inside(staging, &entry.rel)?;
        crate::manifest::assert_inside(staging, &target)?;
        if entry.is_dir {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("create directory {}", target.display()))?;
            continue;
        }
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("create directory {}", dir.display()))?;
        }
        let mut out = std::fs::File::create(&target)
            .with_context(|| format!("create prepared file {}", target.display()))?;
        let mut running = *done;
        // The callback is synchronous, so the progress path can stay borrowed.
        image.stream_to(entry.data_offset, entry.data_len, &mut out, |want| {
            running = running.saturating_add(want);
            emit(progress, &entry.rel, running.min(total), total);
        })?;
        drop(out);
        check_written(&target, entry.data_len, &entry.rel)?;
        if entry.rel.eq_ignore_ascii_case(crate::EXECUTABLE_NAME) {
            crate::manifest::verify_executable(&target, disc)?;
        }
        if entry.rel.eq_ignore_ascii_case(crate::VERSION_FILE_NAME) {
            crate::manifest::verify_version_txt(&target)?;
        }
        *done = done
            .checked_add(entry.data_len)
            .context("progress overflow")?;
        files.push(ManifestFile {
            path: entry.rel.clone(),
            size: entry.data_len,
        });
        emit(progress, &entry.rel, *done, total);
    }
    Ok(())
}

fn emit(progress: &mut impl FnMut(ImportProgress), rel: &str, completed: u64, total: u64) {
    progress(ImportProgress {
        current_file: PathBuf::from(rel),
        completed_bytes: completed,
        total_bytes: total,
    });
}

fn check_written(target: &Path, expected: u64, rel: &str) -> anyhow::Result<()> {
    let actual = std::fs::metadata(target)
        .with_context(|| format!("stat {}", target.display()))?
        .len();
    if actual != expected {
        anyhow::bail!("short write for {rel}: {actual} != {expected}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_system_update;

    #[test]
    fn skips_system_update_tree() {
        assert!(is_system_update("$SystemUpdate/foo.xex"));
        assert!(is_system_update("$systemupdate/nested/deep.bin"));
        assert!(!is_system_update("data/file.big"));
    }
}
