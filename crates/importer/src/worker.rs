use anyhow::Context as _;
use fifa_game_data::{ImportProgress, PreparedGame, import_iso, prepared_game};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::Sender,
};

pub enum WorkerEvent {
    CheckDone(Result<Option<PreparedGame>, String>),
    Progress(ImportProgress),
    ImportDone(Result<PreparedGame, String>),
    BuildProgress(String),
    BuildDone(Result<(), String>),
}

fn notify(tx: &Sender<WorkerEvent>, ctx: &eframe::egui::Context, event: WorkerEvent) {
    if tx.send(event).is_ok() {
        ctx.request_repaint();
    }
}

pub fn spawn_check(destination: PathBuf, tx: Sender<WorkerEvent>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let result = prepared_game(&destination).map_err(|error| format!("{error:#}"));
        notify(&tx, &ctx, WorkerEvent::CheckDone(result));
    });
}

pub fn spawn_import(
    source: PathBuf,
    destination: PathBuf,
    tx: Sender<WorkerEvent>,
    ctx: eframe::egui::Context,
) {
    std::thread::spawn(move || {
        let result = import_iso(&source, &destination, |update| {
            notify(&tx, &ctx, WorkerEvent::Progress(update));
        })
        .map_err(|error| format!("{error:#}"));
        notify(&tx, &ctx, WorkerEvent::ImportDone(result));
    });
}

pub fn spawn_build(game: PathBuf, tx: Sender<WorkerEvent>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let result = build(&game, |line| {
            notify(&tx, &ctx, WorkerEvent::BuildProgress(line));
        })
        .map_err(|error| format!("{error:#}"));
        notify(&tx, &ctx, WorkerEvent::BuildDone(result));
    });
}

fn build(game: &std::path::Path, mut progress: impl FnMut(String)) -> anyhow::Result<()> {
    let installation = crate::paths::exe_dir()?;
    let project = installation.join("project");
    anyhow::ensure!(
        project.join("native/revision.toml").is_file(),
        "Builder project is missing. Extract the complete setup ZIP beside fifa-importer.exe."
    );
    let builder = installation.join("fifa-build.exe");
    anyhow::ensure!(
        builder.is_file(),
        "fifa-build.exe is missing. Extract the complete setup ZIP."
    );
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(installation.join("build.log"))?;
    writeln!(log, "\nStarting local build from {}", game.display())?;
    let mut command = Command::new(builder);
    command
        .arg("--root")
        .arg(&project)
        .arg("--game-dir")
        .arg(game)
        .arg("--install")
        .arg(&installation)
        .current_dir(&project)
        .stdout(Stdio::piped())
        .stderr(log.try_clone()?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().context("Start Windows game builder")?;
    let stdout = child
        .stdout
        .take()
        .context("Builder output pipe is missing")?;
    let read_result = (|| -> anyhow::Result<()> {
        for line in BufReader::new(stdout).lines() {
            let line = line.context("Read builder progress")?;
            writeln!(log, "{line}")?;
            if !line.trim().is_empty() {
                progress(line);
            }
        }
        Ok(())
    })();
    let status = child.wait().context("Wait for Windows game builder")?;
    read_result?;
    anyhow::ensure!(
        status.success(),
        "The local game build failed ({status}). See build.log beside setup. Check that Git, LLVM/Clang 20+ and Visual Studio Desktop development with C++ are installed, then use Check prepared files again to retry."
    );
    anyhow::ensure!(
        installation.join("fifa_world_cup_2006.exe").is_file(),
        "Builder exited without a game executable; see build.log."
    );
    Ok(())
}
