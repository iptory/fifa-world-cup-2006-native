use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}
#[derive(serde::Deserialize)]
struct Package {
    name: String,
    version: String,
    license: Option<String>,
    license_file: Option<String>,
    manifest_path: PathBuf,
}

pub(crate) fn stage(root: &Path, destination: &Path) -> Result<()> {
    let metadata = cargo(
        root,
        &[
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            "x86_64-pc-windows-msvc",
        ],
    )?;
    let metadata: Metadata = serde_json::from_str(&metadata)?;
    let tree = cargo(
        root,
        &[
            "tree",
            "--locked",
            "--target",
            "x86_64-pc-windows-msvc",
            "--edges",
            "normal",
            "--prefix",
            "none",
            "-p",
            "fifa-importer",
            "-p",
            "xtask",
        ],
    )?;
    let mut closure = BTreeSet::new();
    for line in tree.lines() {
        let mut words = line.split_whitespace();
        if let (Some(name), Some(version)) = (words.next(), words.next())
            && let Some(version) = version.strip_prefix('v')
        {
            closure.insert((name, version));
        }
    }
    ensure!(!closure.is_empty(), "Cargo reported no setup dependencies");
    let mut body = String::from(
        "Windows setup and builder dependency notices\n\nThis archive contains Rust setup/build utilities, not a game executable or a compiled ReXGlue runtime. Game code and runtime libraries are built locally.\n\n",
    );
    let mut copied = BTreeSet::new();
    for (name, version) in closure {
        let package = metadata
            .packages
            .iter()
            .find(|p| p.name == name && p.version == version)
            .with_context(|| format!("Missing metadata for {name} {version}"))?;
        writeln!(
            body,
            "{name} {version} ({})",
            package
                .license
                .as_deref()
                .context("Dependency has no license metadata")?
        )?;
        let files = license_files(root, package)?;
        ensure!(!files.is_empty(), "No license text for {name} {version}");
        for file in files {
            if copied.insert(file.clone()) {
                writeln!(
                    body,
                    "\n--- {name} {version}: {} ---\n",
                    file.file_name()
                        .context("License filename missing")?
                        .to_string_lossy()
                )?;
                body.push_str(
                    &fs::read_to_string(&file)
                        .with_context(|| format!("Read {}", file.display()))?,
                );
                body.push('\n');
            }
        }
        body.push('\n');
    }
    let sysroot = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .context("Locate matching Rust standard-library notices")?;
    ensure!(
        sysroot.status.success(),
        "rustc could not report its sysroot"
    );
    let sysroot = String::from_utf8(sysroot.stdout)?;
    let copyright = Path::new(sysroot.trim()).join("share/doc/rust/COPYRIGHT-library.html");
    fs::copy(
        &copyright,
        destination.with_file_name("RUST-STANDARD-LIBRARY.html"),
    )
    .with_context(|| {
        format!(
            "Copy matching standard-library notices from {}",
            copyright.display()
        )
    })?;
    body.push_str("Rust standard-library copyright and license notices are preserved in RUST-STANDARD-LIBRARY.html from the build toolchain. Its upstream inventory includes platform-specific components, not all of which are linked on Windows.\n");
    fs::write(destination, body).context("Write setup dependency notices")
}

fn license_files(root: &Path, package: &Package) -> Result<Vec<PathBuf>> {
    let directory = package
        .manifest_path
        .parent()
        .context("Package has no directory")?;
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let lower = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if entry.file_type()?.is_file()
            && [
                "license",
                "licence",
                "copying",
                "copyright",
                "notice",
                "unlicense",
                "patents",
            ]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
        {
            files.push(entry.path());
        }
    }
    if let Some(file) = &package.license_file {
        files.push(directory.join(file));
    }
    let group = match (package.name.as_str(), package.version.as_str()) {
        ("fifa-importer" | "fifa-game-data" | "xtask", _) => {
            files.push(root.join("LICENSE"));
            None
        }
        (
            "ecolor"
            | "eframe"
            | "egui"
            | "egui-wgpu"
            | "egui-winit"
            | "egui_glow"
            | "emath"
            | "epaint"
            | "epaint_default_fonts",
            "0.33.3",
        ) => Some("egui-0.33.3"),
        ("profiling", "1.0.18") => Some("profiling-1.0.18"),
        ("clipboard-win", "5.4.3") => Some("clipboard-win-5.4.3"),
        ("xdvdfs", "0.8.3") => Some("xdvdfs-0.8.3"),
        ("proc-bitfield-macros", "0.2.3") => Some("proc-bitfield-macros-0.2.3"),
        _ => None,
    };
    if let Some(group) = group {
        for entry in fs::read_dir(root.join("native/licenses").join(group))? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn cargo(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(arguments)
        .output()
        .context("Read locked Cargo dependency inventory")?;
    ensure!(
        output.status.success(),
        "cargo {}: {}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).context("Cargo inventory is not UTF-8")
}
