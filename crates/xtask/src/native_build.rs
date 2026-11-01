use anyhow::{Context, Result};
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn build(
    root: &Path,
    game: &Path,
    sdk: &Path,
    tools: &[(OsString, OsString)],
) -> Result<PathBuf> {
    let native = root.join(".work/native-project");
    fs::create_dir_all(native.join("src"))?;
    for name in [
        "CMakeLists.txt",
        "revision.toml",
        "src/main.cpp",
        "src/fifa_world_cup_2006_app.h",
    ] {
        fs::copy(root.join("native").join(name), native.join(name))?;
    }
    let manifest = write_manifest(&native, game)?;
    crate::platform::run(
        Command::new(sdk.join("bin/rexglue.exe"))
            .current_dir(&native)
            .arg("codegen")
            .arg(manifest),
        tools,
    )?;
    let build = root.join(".work/build-windows-game");
    let compiler = match env::var_os("CXX") {
        Some(compiler) => compiler,
        None => crate::platform::clang_tool("clang-cl")?,
    };
    crate::platform::run(
        Command::new("cmake")
            .current_dir(root)
            .arg("-S")
            .arg(&native)
            .arg("-B")
            .arg(&build)
            .args(["-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release"])
            .arg(format!(
                "-DCMAKE_CXX_COMPILER={}",
                compiler.to_string_lossy()
            ))
            .arg(format!("-DCMAKE_PREFIX_PATH={}", sdk.display()))
            .arg(format!(
                "-Drexglue_DIR={}",
                sdk.join("lib/cmake/rexglue").display()
            ))
            .arg("-DREXSDK_VERSION=0.10.0"),
        tools,
    )?;
    let jobs = std::thread::available_parallelism()?.get().min(6);
    crate::platform::run(
        Command::new("cmake")
            .current_dir(root)
            .arg("--build")
            .arg(&build)
            .arg("--parallel")
            .arg(jobs.to_string()),
        tools,
    )?;
    println!("Built native game: {}", build.display());
    Ok(build)
}

fn write_manifest(native: &Path, game: &Path) -> Result<PathBuf> {
    let quoted = |path: &Path| -> Result<String> {
        Ok(serde_json::to_string(
            &path
                .to_str()
                .context("Build path is not UTF-8")?
                .replace('\\', "/"),
        )?)
    };
    let output = native.join("fifa_world_cup_2006_manifest.toml");
    let text = format!(
        "[project]\nname = \"fifa_world_cup_2006\"\nsdk_version = \"0.10.0\"\ngame_root = {}\n\n[entrypoint]\nfile_path = {}\nout_directory_path = \"generated/default\"\nincludes = [\"revision.toml\"]\n",
        quoted(game)?,
        quoted(&game.join("default.xex"))?
    );
    fs::write(&output, text)?;
    Ok(output)
}
