#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod art;
mod cli;
mod launch;
mod paths;
mod view;
mod worker;

use std::path::PathBuf;

use anyhow::Context as _;

use crate::paths::default_destination;

fn usage() -> &'static str {
    "usage:\n  fifa-importer\n  fifa-importer --inspect <ISO>\n  fifa-importer --import <ISO> --destination <path>"
}

fn arg(index: usize) -> anyhow::Result<String> {
    std::env::args()
        .nth(index)
        .with_context(|| format!("missing argument at position {index}"))
}

fn run_headless() -> anyhow::Result<bool> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        None => Ok(false),
        Some("--inspect") => {
            let iso = PathBuf::from(arg(2)?);
            crate::cli::inspect(&iso)?;
            Ok(true)
        }
        Some("--import") => {
            if arg(3)? != "--destination" {
                anyhow::bail!("expected --destination <path>");
            }
            let iso = PathBuf::from(arg(2)?);
            let destination = PathBuf::from(arg(4)?);
            crate::cli::import(&iso, &destination)?;
            Ok(true)
        }
        Some("--help" | "-h" | "help") => {
            println!("{usage}", usage = usage());
            Ok(true)
        }
        Some(other) => anyhow::bail!("unknown argument: {other}\n{usage}", usage = usage()),
    }
}

fn run_gui() -> anyhow::Result<()> {
    let destination = default_destination()?;
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("2006 FIFA World Cup setup")
            .with_icon(crate::art::icon()?)
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([760.0, 580.0]),
        ..Default::default()
    };
    eframe::run_native(
        "fifa-world-cup-2006-setup",
        options,
        Box::new(move |ctx| {
            Ok(Box::new(crate::app::App::new(
                destination,
                ctx.egui_ctx.clone(),
            )?))
        }),
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}

fn main() -> anyhow::Result<()> {
    if run_headless()? {
        return Ok(());
    }
    run_gui()
}
