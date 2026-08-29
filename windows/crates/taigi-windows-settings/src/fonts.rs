//! The window's CJK fallback: the bundled `jf-openhuninn-2.1.ttf` (the face
//! the input method ships everywhere) appended to egui's proportional
//! family, so Latin keeps egui's own face and every hanji egui lacks comes
//! from the bundle (roadmap W15). No system-font lookup: egui takes bytes,
//! not names, and the bundled face is the one file we know is beside the
//! executable.

// 中文: 視窗字型 — 隨附的 open 粉圓接在 egui 預設字型後面,補漢字。

use taigi_windows_core::settings::CandidateFontChoice;

const FONTS_DIR_NAME: &str = "Fonts";
const FAMILY_NAME: &str = "openhuninn";

pub fn install(ctx: &egui::Context) {
    let Some(file_name) = CandidateFontChoice::OpenHuninn.file_name() else {
        return;
    };
    let Some(directory) = taigi_windows_platform::executable_directory() else {
        log::warn!("fonts.no_executable_directory");
        return;
    };
    let path = directory.join(FONTS_DIR_NAME).join(file_name);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            // A development run outside the install layout: egui's own
            // fonts, which have no CJK glyphs — the labels show as boxes,
            // the window still works.
            log::warn!("fonts.missing path={} error={error}", path.display());
            return;
        }
    };
    let mut definitions = egui::FontDefinitions::default();
    definitions.font_data.insert(
        FAMILY_NAME.to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    // Appended, not prepended: a fallback for the glyphs egui's faces lack,
    // and only in the proportional family — a monospace family with a
    // proportional face at its head would stop being one.
    definitions
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push(FAMILY_NAME.to_owned());
    ctx.set_fonts(definitions);
}
