mod desktop_package;
mod native_build;
mod notices;
mod platform;
mod sdk;
mod setup_package;

use anyhow::{Context, Result, bail, ensure};
use fifa_game_data::SUPPORTED_EXECUTABLE_SHA256;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

fn main() -> Result<()> {
    let mut arguments = env::args_os().skip(1);
    let mut root = None;
    let mut game = None;
    let mut install = None;
    let mut sdk_prefix = env::var_os("REXSDK").map(PathBuf::from);
    let mut setup_package = false;
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--root") => {
                root = Some(PathBuf::from(
                    arguments
                        .next()
                        .context("--root requires a project directory")?,
                ));
            }
            Some("--game-dir") => {
                game = Some(PathBuf::from(
                    arguments
                        .next()
                        .context("--game-dir requires prepared game data")?,
                ));
            }
            Some("--install") => {
                install = Some(PathBuf::from(
                    arguments
                        .next()
                        .context("--install requires a destination")?,
                ));
            }
            Some("--sdk") => {
                sdk_prefix = Some(PathBuf::from(
                    arguments.next().context("--sdk requires an SDK prefix")?,
                ));
            }
            Some("--release-setup") => setup_package = true,
            Some("--help") => {
                println!(
                    "fifa-build [--root <project>] [--game-dir <prepared>] [--install <folder>] [--sdk <prefix>] [--release-setup]"
                );
                println!(
                    "Builds the Windows game locally. --release-setup packages only the Rust setup/builder, project source and notices; never the game executable."
                );
                return Ok(());
            }
            _ => bail!("Unknown argument: {}", argument.to_string_lossy()),
        }
    }
    ensure!(
        cfg!(all(windows, target_arch = "x86_64")),
        "Only Windows x86-64 builds are supported"
    );
    let root = std::path::absolute(root.unwrap_or(env::current_dir()?))?;
    ensure!(
        root.join("native/revision.toml").is_file(),
        "Project source is missing at {}",
        root.display()
    );
    if setup_package {
        return setup_package::package(&root)
            .map(|path| println!("Setup release: {}", path.display()));
    }
    let game = std::path::absolute(game.unwrap_or_else(|| root.join("private/game")))?;
    verify_revision(&game.join("default.xex"))?;
    let tools = platform::environment()?;
    let sdk = match sdk_prefix {
        Some(prefix) => {
            let prefix = std::path::absolute(prefix)?;
            sdk::validate(&root, &prefix)?;
            prefix
        }
        None => sdk::prepare(&root, &tools)?,
    };
    let build = native_build::build(&root, &game, &sdk, &tools)?;
    if let Some(destination) = install {
        desktop_package::install_game(&build, &std::path::absolute(destination)?, &tools)?;
    }
    Ok(())
}

fn verify_revision(game: &Path) -> Result<()> {
    let mut executable = File::open(game).context("Import your supported Xbox 360 ISO first")?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 16384];
    loop {
        let count = executable.read(&mut buffer).context("Read game revision")?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    ensure!(
        format!("{:x}", digest.finalize()) == SUPPORTED_EXECUTABLE_SHA256,
        "Unsupported executable revision in {}",
        game.display()
    );
    Ok(())
}
