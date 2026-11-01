use crate::desktop_package::{copy_file, zip_tree};
use anyhow::{Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn package(root: &Path) -> Result<PathBuf> {
    let tools = crate::platform::environment()?;
    crate::platform::run(
        Command::new("cargo")
            .current_dir(root)
            .env(
                "CARGO_ENCODED_RUSTFLAGS",
                "-C\u{1f}target-feature=+crt-static",
            )
            .args([
                "build",
                "--release",
                "--locked",
                "--target",
                "x86_64-pc-windows-msvc",
                "-p",
                "fifa-importer",
                "-p",
                "xtask",
            ]),
        &tools,
    )?;
    let parent = root.join(".work/dist");
    fs::create_dir_all(&parent)?;
    let output = parent.join(format!(
        "fifa-world-cup-2006-setup-{}-windows-x86_64.zip",
        env!("CARGO_PKG_VERSION")
    ));
    ensure!(
        !output.exists(),
        "Release archive already exists; retain or move it before rebuilding: {}",
        output.display()
    );
    let staging = tempfile::Builder::new()
        .prefix("setup-release-")
        .tempdir_in(&parent)?;
    let dist = staging.path();
    let project = dist.join("project");
    fs::create_dir_all(&project)?;
    for name in ["Cargo.toml", "Cargo.lock", "LICENSE", "README.md"] {
        copy_file(&root.join(name), &project.join(name))?;
    }
    for name in ["game-data", "importer", "xtask"] {
        copy_source(
            &root.join("crates").join(name),
            &project.join("crates").join(name),
        )?;
    }
    copy_source(
        &root.join(".github/images"),
        &project.join(".github/images"),
    )?;
    for name in [
        "CMakeLists.txt",
        "revision.toml",
        "fifa_world_cup_2006_manifest.toml",
    ] {
        copy_file(
            &root.join("native").join(name),
            &project.join("native").join(name),
        )?;
    }
    for directory in ["src", "patches", "licenses"] {
        copy_source(
            &root.join("native").join(directory),
            &project.join("native").join(directory),
        )?;
    }
    let binaries = [
        root.join("target/x86_64-pc-windows-msvc/release/fifa-importer.exe"),
        root.join("target/x86_64-pc-windows-msvc/release/xtask.exe"),
        root.join("target/x86_64-pc-windows-msvc/release/glslang-info.exe"),
    ];
    copy_file(&binaries[0], &dist.join("fifa-importer.exe"))?;
    copy_file(&binaries[1], &dist.join("fifa-build.exe"))?;
    copy_file(&binaries[2], &project.join(".work/tools/glslang-info.exe"))?;
    copy_file(&root.join("LICENSE"), &dist.join("LICENSE"))?;
    crate::notices::stage(root, &dist.join("THIRD_PARTY_LICENSES.txt"))?;
    fs::write(
        dist.join("START-HERE.txt"),
        "2006 FIFA World Cup setup for Windows\n\nExtract this entire ZIP to a writable folder and open fifa-importer.exe.\nDrop your supported Xbox 360 ISO onto the window, or choose it with Browse.\nSetup validates the disc, extracts it locally, and builds the game.\n\nFirst-time game compilation needs Git, LLVM/Clang 20 or newer, and Visual\nStudio with Desktop development with C++. CMake and Ninja can come with\nVisual Studio. Internet access is needed for the pinned runtime sources.\nThe Rust setup and builder are already compiled; Rust is not needed to use\nthis downloaded package. Compilation can take a while. The window shows\nthe current build output; build.log keeps errors and command output.\n\nAfter setup, the game executable launches independently. Keep its runtime\nDLLs, prepared/ and project/ beside the setup files. Never redistribute\nthe game executable, generated source, extracted assets or your disc image.\n\nThis is an unofficial compatibility project. Windows gameplay has limited\nverification; full mode coverage, uncapped rendering and broad hardware\ncompatibility are not claimed. See project/README.md for the exact revision.\n\nProject source is MIT-licensed; dependencies retain their own licenses.\nArtwork was supplied by the repository owner for this setup. EA/FIFA and\nother trademarks remain their owners'. No affiliation or endorsement is\nclaimed. Owning a disc does not guarantee that copying, circumvention or\nrecompilation is permitted in your jurisdiction. No legal immunity is claimed.\n",
    )?;
    audit(dist, &project)?;
    zip_tree(dist, &output)?;
    Ok(output)
}

fn copy_source(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let text = name.to_string_lossy();
        if text.starts_with("android")
            || text.contains("-android")
            || text == "linux-uapi"
            || text.starts_with("jni-")
            || text.starts_with("ndk")
            || text == "target"
        {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_source(&path, &destination.join(name))?;
        } else {
            ensure!(kind.is_file(), "Unexpected source entry {}", path.display());
            copy_file(&path, &destination.join(name))?;
        }
    }
    Ok(())
}

fn audit(dist: &Path, project: &Path) -> Result<()> {
    let allowed = [
        dist.join("fifa-importer.exe"),
        dist.join("fifa-build.exe"),
        project.join(".work/tools/glslang-info.exe"),
    ];
    let mut pending = vec![dist.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            ensure!(
                !path.strip_prefix(dist)?.components().any(|component| [
                    "generated",
                    "prepared",
                    "private",
                    "user",
                    ".git"
                ]
                .iter()
                .any(|name| component.as_os_str() == *name)),
                "Private/generated directory in release: {}",
                path.display()
            );
            if entry.file_type()?.is_dir() {
                pending.push(path);
                continue;
            }
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            ensure!(
                ![
                    "iso", "xex", "xexp", "obj", "o", "lib", "a", "pdb", "apk", "so", "wmv", "xma",
                    "pem", "key", "pfx", "keystore", "dll"
                ]
                .contains(&extension.as_str()),
                "Private/build artifact in release: {}",
                path.display()
            );
            if extension == "exe" {
                ensure!(
                    allowed.contains(&path),
                    "Unapproved executable in release: {}",
                    path.display()
                );
            }
        }
    }
    println!(
        "Audited setup-only release: no game executable, game data, or generated recompilation output."
    );
    Ok(())
}

#[cfg(test)]
#[path = "release_checks.rs"]
mod tests;
