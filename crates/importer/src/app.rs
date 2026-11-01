use crate::{art::Art, worker::WorkerEvent};
use eframe::egui;
use fifa_game_data::{ImportProgress, PreparedGame};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

pub(crate) enum Phase {
    Checking,
    NeedIso,
    Importing {
        current: PathBuf,
        completed: u64,
        total: u64,
    },
    NeedBuild {
        game: PreparedGame,
    },
    Building {
        game: PreparedGame,
    },
    Ready {
        game: PreparedGame,
    },
    Failed,
}

pub(crate) enum Action {
    None,
    Choose,
    Build,
    Launch,
    Recheck,
}

pub struct App {
    pub(crate) destination: PathBuf,
    rx: Receiver<WorkerEvent>,
    tx: Sender<WorkerEvent>,
    pub(crate) phase: Phase,
    pub(crate) status: String,
    pub(crate) art: Art,
}

impl App {
    pub fn new(destination: PathBuf, ctx: egui::Context) -> anyhow::Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let art = Art::new(&ctx)?;
        let app = Self {
            destination: destination.clone(),
            rx,
            tx: tx.clone(),
            phase: Phase::Checking,
            status: "Checking your prepared files…".into(),
            art,
        };
        crate::worker::spawn_check(destination, tx, ctx);
        Ok(app)
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                WorkerEvent::CheckDone(result) => match result {
                    Ok(Some(game)) => self.prepared(game),
                    Ok(None) => {
                        self.phase = Phase::NeedIso;
                        self.status = "Use your own Xbox 360 disc image.".into();
                    }
                    Err(message) => self.fail(message),
                },
                WorkerEvent::Progress(ImportProgress {
                    current_file,
                    completed_bytes,
                    total_bytes,
                }) => {
                    if let Phase::Importing {
                        current,
                        completed,
                        total,
                    } = &mut self.phase
                    {
                        *current = current_file;
                        *completed = completed_bytes;
                        *total = total_bytes;
                    }
                }
                WorkerEvent::ImportDone(result) => match result {
                    Ok(game) => {
                        self.prepared(game);
                        if matches!(self.phase, Phase::NeedBuild { .. }) {
                            self.build(ctx);
                        }
                    }
                    Err(message) => self.fail(message),
                },
                WorkerEvent::BuildProgress(line) => self.status = line,
                WorkerEvent::BuildDone(result) => match result {
                    Ok(()) => {
                        let old = std::mem::replace(&mut self.phase, Phase::Checking);
                        if let Phase::Building { game } = old {
                            self.phase = Phase::Ready { game };
                            self.status = "Your game is ready.".into();
                        }
                    }
                    Err(message) => self.fail(message),
                },
            }
        }
    }

    fn prepared(&mut self, game: PreparedGame) {
        match crate::launch::game_executable() {
            Ok(path) if path.is_file() => {
                self.phase = Phase::Ready { game };
                self.status = "Your game is ready.".into();
            }
            Ok(_) => {
                self.phase = Phase::NeedBuild { game };
                self.status = "Disc files are ready. Build the Windows game once.".into();
            }
            Err(error) => self.fail(format!("{error:#}")),
        }
    }

    fn fail(&mut self, message: String) {
        self.status = message;
        self.phase = Phase::Failed;
    }

    fn import(&mut self, source: PathBuf, ctx: &egui::Context) {
        if !source.is_file()
            || !source
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("iso"))
        {
            self.fail("Choose one .iso file, not a folder or shortcut.".into());
            return;
        }
        self.status = "Verifying the disc revision before extraction…".into();
        self.phase = Phase::Importing {
            current: PathBuf::new(),
            completed: 0,
            total: 0,
        };
        crate::worker::spawn_import(
            source,
            self.destination.clone(),
            self.tx.clone(),
            ctx.clone(),
        );
    }

    fn build(&mut self, ctx: &egui::Context) {
        let old = std::mem::replace(&mut self.phase, Phase::Checking);
        match old {
            Phase::NeedBuild { game } => {
                crate::worker::spawn_build(game.root.clone(), self.tx.clone(), ctx.clone());
                self.phase = Phase::Building { game };
                self.status =
                    "Starting the local Windows build. The first build downloads runtime sources."
                        .into();
            }
            other => self.phase = other,
        }
    }

    fn launch(&mut self, ctx: &egui::Context) {
        if let Phase::Ready { game } = &self.phase {
            match crate::launch::launch(&game.root) {
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error) => self.fail(format!("{error:#}")),
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.poll(ctx);
        if matches!(
            self.phase,
            Phase::NeedIso | Phase::Failed | Phase::NeedBuild { .. }
        ) {
            let dropped = ctx.input(|input| input.raw.dropped_files.clone());
            if !dropped.is_empty() {
                match dropped.as_slice() {
                    [file] => match &file.path {
                        Some(path) => self.import(path.clone(), ctx),
                        None => self.fail(
                            "Windows did not provide a local path for the dropped file.".into(),
                        ),
                    },
                    _ => self.fail("Drop one ISO at a time.".into()),
                }
            }
        }
        match crate::view::draw(self, ctx) {
            Action::None => {}
            Action::Choose => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_parent(frame)
                    .set_title("Choose your Xbox 360 disc image")
                    .add_filter("Xbox 360 ISO", &["iso"])
                    .pick_file()
                {
                    self.import(path, ctx);
                }
            }
            Action::Build => self.build(ctx),
            Action::Launch => self.launch(ctx),
            Action::Recheck => {
                self.phase = Phase::Checking;
                self.status = "Checking your prepared files…".into();
                crate::worker::spawn_check(self.destination.clone(), self.tx.clone(), ctx.clone());
            }
        }
    }
}
