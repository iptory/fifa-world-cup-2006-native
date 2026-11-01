use std::{ffi::OsString, process::Command};

use anyhow::{Context, Result, ensure};

pub(crate) fn environment() -> Result<Vec<(OsString, OsString)>> {
    #[cfg(windows)]
    {
        use std::{env, os::windows::process::CommandExt, path::PathBuf};

        let installer =
            PathBuf::from(env::var_os("ProgramFiles(x86)").context("ProgramFiles(x86) is unset")?)
                .join("Microsoft Visual Studio/Installer/vswhere.exe");
        let found = Command::new(&installer)
            .args([
                "-latest",
                "-prerelease",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ])
            .output()
            .context("Locate Visual Studio with the Desktop development with C++ workload")?;
        ensure!(
            found.status.success(),
            "vswhere failed: {}",
            String::from_utf8_lossy(&found.stderr)
        );
        let installation =
            String::from_utf8(found.stdout).context("Visual Studio path is not UTF-8")?;
        ensure!(
            !installation.trim().is_empty(),
            "Install Visual Studio Desktop development with C++"
        );
        let vcvars = PathBuf::from(installation.trim()).join("VC/Auxiliary/Build/vcvars64.bat");
        let path = vcvars
            .to_str()
            .context("Visual Studio environment path is not UTF-8")?;
        ensure!(
            !path.contains(['"', '%', '\r', '\n']),
            "Unsupported character in Visual Studio path"
        );
        let output = Command::new("cmd.exe")
            .raw_arg(format!(r#"/d /s /c "call "{path}" && set""#))
            .output()
            .context("Initialize the MSVC compiler environment")?;
        ensure!(
            output.status.success(),
            "vcvars64 failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).context("Compiler environment is not UTF-8")?;
        let mut values = Vec::new();
        for line in text.lines() {
            if let Some((name, value)) = line.split_once('=')
                && !name.is_empty()
            {
                values.push((OsString::from(name), OsString::from(value)));
            }
        }
        if let Some((_, value)) = values
            .iter_mut()
            .find(|(name, _)| name.to_string_lossy().eq_ignore_ascii_case("PATH"))
        {
            let installation = PathBuf::from(installation.trim());
            let mut paths: Vec<_> = env::split_paths(value).collect();
            paths.push(installation.join("Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin"));
            paths.push(installation.join("Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja"));
            *value = env::join_paths(paths).context("Add Visual Studio build tools to PATH")?;
        }
        Ok(values)
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

pub(crate) fn run(command: &mut Command, environment: &[(OsString, OsString)]) -> Result<()> {
    command.envs(environment.iter().map(|(name, value)| (name, value)));
    let status = command
        .status()
        .with_context(|| format!("Run {}", command.get_program().to_string_lossy()))?;
    ensure!(
        status.success(),
        "{} failed with {status}",
        command.get_program().to_string_lossy()
    );
    Ok(())
}

pub(crate) fn clang_tool(name: &str) -> Result<OsString> {
    let override_name = match name {
        "clang" => Some("CC"),
        "clang++" => Some("CXX"),
        _ => None,
    };
    if let Some(command) = override_name.and_then(std::env::var_os) {
        return Ok(command);
    }
    #[cfg(windows)]
    {
        use std::{env, path::PathBuf};
        Ok(
            PathBuf::from(env::var_os("ProgramFiles").context("ProgramFiles is unset")?)
                .join("LLVM/bin")
                .join(format!("{name}.exe"))
                .into_os_string(),
        )
    }
    #[cfg(not(windows))]
    {
        Ok(OsString::from(name))
    }
}
