use std::path::{Path, PathBuf};

use anyhow::Context as _;

use crate::paths::{absolute, exe_dir};

pub(crate) fn game_executable() -> anyhow::Result<PathBuf> {
    Ok(exe_dir()?.join("fifa_world_cup_2006.exe"))
}

/// # Errors
/// Fails if the sibling game is missing or cannot start; setup need not stay open.
pub fn launch(prepared_root: &Path) -> anyhow::Result<()> {
    let exe = game_executable()?;
    if !exe.is_file() {
        anyhow::bail!("game executable not found: {}", exe.display());
    }
    let root = absolute(prepared_root)?;
    std::process::Command::new(&exe)
        .arg("--game_data_root")
        .arg(&root)
        .spawn()
        .with_context(|| format!("launching {}", exe.display()))?;
    Ok(())
}
