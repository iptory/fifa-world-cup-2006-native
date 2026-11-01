use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CRT: &[&str] = &[
    "concrt140.dll",
    "msvcp140.dll",
    "msvcp140_1.dll",
    "msvcp140_2.dll",
    "msvcp140_atomic_wait.dll",
    "msvcp140_codecvt_ids.dll",
    "vccorlib140.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "vcruntime140_threads.dll",
];

pub(crate) fn install_game(
    build: &Path,
    destination: &Path,
    tools: &[(OsString, OsString)],
) -> Result<()> {
    fs::create_dir_all(destination)?;
    let binaries: Vec<_> = [
        "fifa_world_cup_2006.exe",
        "rexruntime.dll",
        "rexgpu-xenos.dll",
    ]
    .iter()
    .map(|name| build.join(name))
    .collect();
    for binary in &binaries {
        copy_file(
            binary,
            &destination.join(binary.file_name().context("Missing binary name")?),
        )?;
    }
    stage_windows_crt(destination, &binaries, tools)?;
    println!("Installed game at {}", destination.display());
    Ok(())
}

pub(crate) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    ensure!(source.is_file(), "Missing {}", source.display());
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)
        .with_context(|| format!("Copy {} to {}", source.display(), destination.display()))?;
    Ok(())
}

pub(crate) fn stage_windows_crt(
    destination: &Path,
    binaries: &[PathBuf],
    tools: &[(OsString, OsString)],
) -> Result<()> {
    let tool = crate::platform::clang_tool("llvm-readobj")?;
    let mut required = std::collections::BTreeSet::new();
    for binary in binaries {
        let output = Command::new(&tool)
            .arg("--coff-imports")
            .arg(binary)
            .output()
            .with_context(|| format!("Inspect {}", binary.display()))?;
        ensure!(
            output.status.success(),
            "Import inspection failed for {}",
            binary.display()
        );
        let text = String::from_utf8(output.stdout)?;
        ensure!(
            text.contains("Import {"),
            "No import records in {}",
            binary.display()
        );
        for line in text.lines() {
            if let Some(name) = line.trim().strip_prefix("Name:") {
                let name = name.trim().to_ascii_lowercase();
                if CRT.contains(&name.as_str()) {
                    required.insert(name);
                }
            }
        }
    }
    if required.is_empty() {
        return Ok(());
    }
    let redist = tools
        .iter()
        .find(|(name, _)| {
            name.to_string_lossy()
                .eq_ignore_ascii_case("VCToolsRedistDir")
        })
        .map(|(_, value)| PathBuf::from(value))
        .context("MSVC redistributable directory is unavailable")?;
    let x64 = redist.join("x64");
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&x64).with_context(|| format!("Read {}", x64.display()))? {
        let path = entry?.path();
        if path.is_dir() && required.iter().all(|name| path.join(name).is_file()) {
            candidates.push(path);
        }
    }
    candidates.sort();
    let source = candidates
        .last()
        .context("Required MSVC runtime files are not installed")?;
    for name in required {
        copy_file(&source.join(&name), &destination.join(name))?;
    }
    Ok(())
}

pub(crate) fn zip_tree(root: &Path, output: &Path) -> Result<()> {
    let mut writer = zip::ZipWriter::new(fs::File::create(output)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6));
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let kind = entry.file_type()?;
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else {
                ensure!(kind.is_file(), "Cannot archive {}", path.display());
                let name = path
                    .strip_prefix(root)?
                    .to_str()
                    .context("Non-UTF-8 archive path")?
                    .replace('\\', "/");
                writer.start_file(name, options)?;
                std::io::copy(&mut fs::File::open(path)?, &mut writer)?;
            }
        }
    }
    writer.finish()?.sync_all()?;
    Ok(())
}
