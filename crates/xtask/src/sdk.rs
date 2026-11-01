use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

use crate::platform;

pub(crate) const REVISION: &str = "f5337cdc947ff6d4c4196737e2c807a48f2a1fc2";
pub(crate) const DEPENDENCIES: &[&str] = &[
    "libmspack",
    "glslang",
    "FFmpeg",
    "tomlplusplus",
    "simde",
    "xxHash",
    "spdlog",
    "fmt",
    "utfcpp",
    "vulkan-headers",
    "vulkan-memory-allocator",
    "imgui",
    "spirv-tools",
    "spirv-headers",
    "cli11",
    "o1heap",
    "sdl3",
    "inja",
    "tracy",
];

fn fingerprint(root: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(REVISION);
    for path in [
        "native/patches/rexglue-0.10.0-runtime.patch",
        "native/patches/rexglue-0.10.0-timer-deadlines.patch",
        "native/patches/rexglue-0.10.0-exception-forwarding.patch",
        "native/patches/rexglue-0.10.0-posix-waits.patch",
        "native/patches/rexglue-0.10.0-sdl-quit.patch",
        "native/patches/glslang-rust-info.patch",
        "crates/xtask/src/bin/glslang-info.rs",
    ] {
        hash.update(fs::read(root.join(path)).with_context(|| format!("Read {path}"))?);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) fn validate(root: &Path, prefix: &Path) -> Result<()> {
    let marker = fs::read_to_string(prefix.join("fifa-runtime-patch.sha256")).context(
        "The SDK needs the runtime repairs. Run xtask without --sdk to build it locally",
    )?;
    ensure!(
        marker.trim() == fingerprint(root)?,
        "SDK repairs differ from this source tree; rebuild without --sdk"
    );
    Ok(())
}

pub(crate) fn apply(source: &Path, patch: &Path, tools: &[(OsString, OsString)]) -> Result<()> {
    let applied = Command::new("git")
        .current_dir(source)
        .args(["apply", "--reverse", "--check"])
        .arg(patch)
        .output()
        .context("Check whether the runtime repair is already applied")?;
    if !applied.status.success() {
        platform::run(
            Command::new("git")
                .current_dir(source)
                .arg("apply")
                .arg(patch),
            tools,
        )?;
    }
    Ok(())
}

pub(crate) fn prepare(root: &Path, tools: &[(OsString, OsString)]) -> Result<PathBuf> {
    ensure!(
        cfg!(all(windows, target_arch = "x86_64")),
        "Only Windows x86-64 SDK builds are supported"
    );
    let platform_name = "win-amd64";
    let prefix = root.join(".work/sdk-patched").join(platform_name);
    if validate(root, &prefix).is_ok() {
        return Ok(prefix);
    }
    let source = prepare_sources(root, tools)?;

    let metadata = prepare_metadata(root, tools)?;
    let build_name = platform_name
        .strip_suffix("-amd64")
        .unwrap_or(platform_name);
    let build = root.join(".work").join(format!("sdk-build-{build_name}"));
    let mut configure = Command::new("cmake");
    configure
        .arg("-S")
        .arg(&source)
        .arg("-B")
        .arg(&build)
        .args(["-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release"])
        .arg(format!(
            "-DCMAKE_C_COMPILER={}",
            platform::clang_tool("clang")?.to_string_lossy()
        ))
        .arg(format!(
            "-DCMAKE_CXX_COMPILER={}",
            platform::clang_tool("clang++")?.to_string_lossy()
        ))
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()))
        .arg(format!("-DGLSLANG_INFO_EXECUTABLE={}", metadata.display()))
        .args([
            "-DREXGLUE_BUILD_TESTS=OFF",
            "-DREXGLUE_ENABLE_TRACY=OFF",
            "-DREXGLUE_ENABLE_FIDELITYFX=OFF",
            "-DCMAKE_DISABLE_FIND_PACKAGE_Python=ON",
            "-DCMAKE_DISABLE_FIND_PACKAGE_Python3=ON",
            "-DCMAKE_DISABLE_FIND_PACKAGE_PythonInterp=ON",
        ]);
    if cfg!(windows) {
        configure.arg("-DREXGLUE_USE_VULKAN=OFF");
    }
    if std::env::consts::ARCH == "x86_64" {
        configure.args([
            "-DCMAKE_C_FLAGS=-march=x86-64-v2",
            "-DCMAKE_CXX_FLAGS=-march=x86-64-v2",
        ]);
    }
    platform::run(&mut configure, tools)?;
    let jobs = std::thread::available_parallelism()?.get().min(2);
    platform::run(
        Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .arg("--parallel")
            .arg(jobs.to_string()),
        tools,
    )?;
    platform::run(Command::new("cmake").arg("--install").arg(&build), tools)?;
    fs::write(prefix.join("fifa-runtime-patch.sha256"), fingerprint(root)?)?;
    Ok(prefix)
}

pub(crate) fn prepare_metadata(root: &Path, tools: &[(OsString, OsString)]) -> Result<PathBuf> {
    let source = root.join("crates/xtask/src/bin/glslang-info.rs");
    let metadata = root.join(".work/tools").join(if cfg!(windows) {
        "glslang-info.exe"
    } else {
        "glslang-info"
    });
    if metadata.is_file()
        && fs::metadata(&metadata)?.modified()? >= fs::metadata(&source)?.modified()?
    {
        return Ok(metadata);
    }
    fs::create_dir_all(metadata.parent().context("Locate SDK tool directory")?)?;
    platform::run(
        Command::new("rustc")
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&metadata),
        tools,
    )?;
    Ok(metadata)
}

pub(crate) fn prepare_sources(root: &Path, tools: &[(OsString, OsString)]) -> Result<PathBuf> {
    let source = root.join(".work/rexglue-source");
    if !source.exists() {
        fs::create_dir_all(root.join(".work"))?;
        platform::run(
            Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    "--branch",
                    "v0.10.0",
                    "https://github.com/rexglue/rexglue-sdk.git",
                ])
                .arg(&source),
            tools,
        )?;
    }
    let revision = Command::new("git")
        .current_dir(&source)
        .args(["rev-parse", "HEAD"])
        .output()?;
    ensure!(
        revision.status.success() && String::from_utf8_lossy(&revision.stdout).trim() == REVISION,
        "SDK source is not the pinned v0.10.0 revision at {}",
        source.display()
    );
    platform::run(
        Command::new("git")
            .current_dir(&source)
            .args([
                "submodule",
                "update",
                "--init",
                "--depth",
                "1",
                "--jobs",
                "6",
                "--",
            ])
            .args(DEPENDENCIES.iter().map(|name| format!("thirdparty/{name}"))),
        tools,
    )?;
    apply(
        &source,
        &root.join("native/patches/rexglue-0.10.0-runtime.patch"),
        tools,
    )?;
    apply(
        &source,
        &root.join("native/patches/rexglue-0.10.0-timer-deadlines.patch"),
        tools,
    )?;
    apply(
        &source,
        &root.join("native/patches/rexglue-0.10.0-exception-forwarding.patch"),
        tools,
    )?;
    apply(
        &source,
        &root.join("native/patches/rexglue-0.10.0-posix-waits.patch"),
        tools,
    )?;
    apply(
        &source,
        &root.join("native/patches/rexglue-0.10.0-sdl-quit.patch"),
        tools,
    )?;
    apply(
        &source.join("thirdparty/glslang"),
        &root.join("native/patches/glslang-rust-info.patch"),
        tools,
    )?;
    Ok(source)
}
