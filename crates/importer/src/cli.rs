use std::path::Path;

use anyhow::Context as _;

use fifa_game_data::{import_iso, inspect_iso};

/// # Errors
/// Rejects unreadable images and unsupported revisions.
pub fn inspect(iso: &Path) -> anyhow::Result<()> {
    let disc = inspect_iso(iso).context("inspecting ISO")?;
    let title = disc.title_id;
    let media = disc.media_id;
    let version = disc.version;
    let sha = disc.executable_sha256;
    println!("title_id={title:08X}");
    println!("media_id={media:08X}");
    println!("version={version}");
    println!("executable_sha256={sha}");
    Ok(())
}

/// # Errors
/// Reports inspection, extraction or validation failures; files are extracted, not the ISO itself.
pub fn import(iso: &Path, destination: &Path) -> anyhow::Result<()> {
    let game = import_iso(iso, destination, |update| {
        let current = update.current_file.display();
        let completed = update.completed_bytes;
        let total = update.total_bytes;
        eprintln!("{current} ({completed}/{total} bytes)");
    })
    .context("importing ISO")?;
    let root = game.root.display();
    let title = game.disc.title_id;
    let media = game.disc.media_id;
    let version = game.disc.version;
    println!("prepared_root={root}");
    println!("title_id={title:08X}");
    println!("media_id={media:08X}");
    println!("version={version}");
    Ok(())
}
