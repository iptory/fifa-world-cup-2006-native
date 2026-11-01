use crate::{
    app::{Action, App, Phase},
    art::{GOLD, INK},
};
use eframe::egui::{self, Color32, RichText, Stroke, StrokeKind};

pub(crate) fn draw(app: &App, ctx: &egui::Context) -> Action {
    let mut action = Action::None;
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(INK)).show(ctx, |ui| {
        let bounds = ui.max_rect();
        let painter = ui.painter();
        let image_aspect = 1.5;
        let shown_aspect = bounds.width() / bounds.height();
        let uv_height = (image_aspect / shown_aspect).min(1.0);
        let uv_width = (shown_aspect / image_aspect).min(1.0);
        let uv = egui::Rect::from_min_max(egui::pos2((1.0 - uv_width) / 2.0, (1.0 - uv_height) / 2.0),
            egui::pos2(f32::midpoint(1.0, uv_width), f32::midpoint(1.0, uv_height)));
        painter.image(app.art.background.id(), bounds, uv, Color32::WHITE);
        let width = (bounds.width() * 0.49).clamp(350.0, 470.0);
        let panel = egui::Rect::from_min_size(bounds.min, egui::vec2(width + 36.0, bounds.height()));
        painter.rect_filled(panel, 0.0, Color32::from_black_alpha(210));
        let logo_width = width - 16.0;
        let logo = egui::Rect::from_min_size(bounds.min + egui::vec2(26.0, 25.0), egui::vec2(logo_width, logo_width * 809.0 / 1942.0));
        painter.image(app.art.logo.id(), logo, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
        let content = egui::Rect::from_min_max(egui::pos2(bounds.left() + 32.0, logo.bottom() + 24.0),
            egui::pos2(bounds.left() + width + 8.0, bounds.bottom() - 22.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(RichText::new("Windows setup").size(25.0).strong());
                ui.label(RichText::new("Unofficial Xbox 360 native port").size(13.0).color(Color32::from_gray(200)));
                ui.add_space(12.0);
                match &app.phase {
                    Phase::NeedIso | Phase::Failed => {
                        ui.label("Prepare your own copy of the game. Nothing from the disc is downloaded or shared.");
                        ui.add_space(8.0);
                        let hovering = ctx.input(|input| !input.raw.hovered_files.is_empty());
                        let response = ui.add_sized([ui.available_width(), 112.0],
                            egui::Button::new(RichText::new(if hovering { "Release to import your ISO" } else { "Drop your Xbox 360 ISO here\nor click to browse" }).size(17.0))
                                .fill(if hovering { Color32::from_rgb(56, 47, 24) } else { INK })
                                .stroke(Stroke::new(if hovering { 2.0_f32 } else { 1.0_f32 }, GOLD)));
                        if response.clicked() { action = Action::Choose; }
                        ui.label(RichText::new("Supported disc: 83769PAL_En • title 454107E6").size(12.0).color(Color32::from_gray(190)));
                        if matches!(app.phase, Phase::Failed) {
                            ui.add_space(6.0);
                            ui.label(RichText::new(&app.status).color(Color32::from_rgb(255, 181, 167)));
                            if ui.button("Check prepared files again").clicked() { action = Action::Recheck; }
                        }
                    }
                    Phase::Checking => { ui.label(&app.status); }
                    Phase::Importing { current, completed, total } => {
                        ui.label(&app.status);
                        if *total != 0 {
                            let percent = u16::try_from(u128::from((*completed).min(*total)) * 10_000 / u128::from(*total));
                            if let Ok(percent) = percent {
                                ui.add(egui::ProgressBar::new(f32::from(percent) / 10_000.0).fill(GOLD).show_percentage().animate(false));
                            }
                            ui.label(format!("{} / {} MiB copied", completed / 1_048_576, total / 1_048_576));
                            ui.label(RichText::new(current.to_string_lossy()).size(12.0));
                        }
                    }
                    Phase::NeedBuild { .. } => {
                        ui.label(&app.status);
                        ui.label("The first build needs Git, LLVM/Clang 20+ and Visual Studio with the C++ desktop workload. It downloads the pinned runtime sources, then compiles locally.");
                        if primary(ui, "Build Windows game") { action = Action::Build; }
                    }
                    Phase::Building { .. } => {
                        ui.label("Building on this PC");
                        ui.label("The first build can take a while. You can leave this window open; there is no game download.");
                        ui.add_space(8.0);
                        ui.label(RichText::new(&app.status).size(12.0));
                        ui.label(RichText::new("Full output is saved in build.log beside setup.").size(12.0).color(Color32::from_gray(190)));
                    }
                    Phase::Ready { .. } => {
                        ui.label(&app.status);
                        ui.label("You can open the game directly next time. Setup closes after launching it.");
                        if primary(ui, "Play 2006 FIFA World Cup") { action = Action::Launch; }
                    }
                }
                ui.add_space(18.0);
                ui.label(RichText::new("Your files stay on this PC.").size(12.0).color(Color32::from_gray(190)));
                ui.label(RichText::new(format!("Data: {}", app.destination.display())).size(11.0).color(Color32::from_gray(180)));
            });
        });
        if ctx.input(|input| !input.raw.hovered_files.is_empty())
            && matches!(app.phase, Phase::NeedIso | Phase::Failed | Phase::NeedBuild { .. }) {
            ui.painter().rect_stroke(bounds.shrink(2.0), 0.0, Stroke::new(3.0_f32, GOLD), StrokeKind::Inside);
        }
    });
    action
}

fn primary(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add_sized(
        [ui.available_width(), 48.0],
        egui::Button::new(RichText::new(label).size(16.0).strong().color(INK)).fill(GOLD),
    )
    .clicked()
}
