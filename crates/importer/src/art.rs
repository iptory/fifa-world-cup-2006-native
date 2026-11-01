use anyhow::Context as _;
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};

pub(crate) const GOLD: Color32 = Color32::from_rgb(240, 194, 77);
pub(crate) const INK: Color32 = Color32::from_rgb(12, 17, 23);

pub(crate) struct Art {
    pub background: TextureHandle,
    pub logo: TextureHandle,
}

impl Art {
    pub fn new(ctx: &egui::Context) -> anyhow::Result<Self> {
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.override_text_color = Some(Color32::from_rgb(244, 242, 234));
        style.visuals.selection.bg_fill = GOLD;
        style.visuals.selection.stroke.color = INK;
        style.spacing.item_spacing = egui::vec2(10.0, 12.0);
        style.spacing.button_padding = egui::vec2(22.0, 12.0);
        ctx.set_style(style);
        Ok(Self {
            background: texture(
                ctx,
                "background",
                include_bytes!("../assets/builderbackground.png"),
            )?,
            logo: texture(ctx, "logo", include_bytes!("../assets/builderlogo.png"))?,
        })
    }
}

fn texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> anyhow::Result<TextureHandle> {
    let image = image::load_from_memory(bytes)
        .context("Decode setup artwork")?
        .into_rgba8();
    let size = [
        usize::try_from(image.width())?,
        usize::try_from(image.height())?,
    ];
    Ok(ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        TextureOptions::LINEAR,
    ))
}

pub(crate) fn icon() -> anyhow::Result<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../assets/builderappicon.png"))?
        .resize(128, 128, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    Ok(egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}
