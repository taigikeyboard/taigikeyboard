//! The window's faces, in this order in egui's proportional family: the
//! system's UI face first (Segoe UI Variable, Segoe UI on Windows 10 — the
//! settings window is a Windows window, so Latin is drawn in what every
//! other Windows window draws it in), egui's own faces after it, and the
//! bundled `jf-openhuninn-2.1.ttf` (the face the input method ships
//! everywhere) appended for every hanji the two before it lack (roadmap
//! W15). Plus the system's icon face (Segoe Fluent Icons, Segoe MDL2 Assets
//! on Windows 10) as its own family for the sidebar glyphs. egui takes
//! bytes, not names, so each is read from `%WINDIR%\Fonts` / the install
//! directory; whichever is missing is skipped and the window still works.

// 中文: 視窗字型 — 系統 UI 字型在前、egui 內建居中、隨附 open 粉圓在後補漢字;另載系統 icon 字型給側欄。

use taigi_windows_core::settings::CandidateFontChoice;
use taigi_windows_platform::{executable_directory, system_fonts_directory};

const FONTS_DIR_NAME: &str = "Fonts";
const CJK_FAMILY_NAME: &str = "openhuninn";
const SYSTEM_FAMILY_NAME: &str = "segoe-ui";
/// The family the sidebar draws its glyphs in — see [`icon_family`].
const ICON_FAMILY_NAME: &str = "segoe-icons";
/// Windows 11's variable face, then Windows 10's.
const SYSTEM_UI_FILES: [&str; 2] = ["SegUIVar.ttf", "segoeui.ttf"];
/// Segoe Fluent Icons (Windows 11), then Segoe MDL2 Assets (Windows 10);
/// the glyphs the sidebar uses have the same code points in both.
const ICON_FILES: [&str; 2] = ["SegoeIcons.ttf", "segmdl2.ttf"];

/// The egui family the system icon face is registered under.
pub fn icon_family() -> egui::FontFamily {
    egui::FontFamily::Name(ICON_FAMILY_NAME.into())
}

/// What [`install`] managed to load; the sidebar draws icons only when the
/// icon face is there (egui would draw boxes otherwise).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InstalledFonts {
    pub has_icon_face: bool,
}

pub fn install(ctx: &egui::Context) -> InstalledFonts {
    let mut definitions = egui::FontDefinitions::default();
    let mut installed = InstalledFonts::default();
    let system_fonts = system_fonts_directory();

    if let Some(bytes) = system_fonts
        .as_ref()
        .and_then(|directory| read_first(directory, &SYSTEM_UI_FILES))
    {
        definitions.font_data.insert(
            SYSTEM_FAMILY_NAME.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        // Prepended: Latin and digits come from the system face; egui's own
        // faces stay behind it for whatever it lacks.
        definitions
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, SYSTEM_FAMILY_NAME.to_owned());
    }

    if let Some(bytes) = system_fonts
        .as_ref()
        .and_then(|directory| read_first(directory, &ICON_FILES))
    {
        definitions.font_data.insert(
            ICON_FAMILY_NAME.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        definitions
            .families
            .insert(icon_family(), vec![ICON_FAMILY_NAME.to_owned()]);
        installed.has_icon_face = true;
    }

    if let Some(bytes) = bundled_cjk_face() {
        definitions.font_data.insert(
            CJK_FAMILY_NAME.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        // Appended, not prepended: a fallback for the glyphs the faces
        // before it lack, and only in the proportional family — a monospace
        // family with a proportional face at its head would stop being one.
        definitions
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push(CJK_FAMILY_NAME.to_owned());
    }

    ctx.set_fonts(definitions);
    installed
}

/// The bundled CJK face beside the executable. A development run outside
/// the install layout has none: egui's own fonts, which have no CJK glyphs
/// — the labels show as boxes, the window still works.
fn bundled_cjk_face() -> Option<Vec<u8>> {
    let file_name = CandidateFontChoice::OpenHuninn.file_name()?;
    let Some(directory) = executable_directory() else {
        log::warn!("fonts.no_executable_directory");
        return None;
    };
    let path = directory.join(FONTS_DIR_NAME).join(file_name);
    match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            log::warn!("fonts.missing path={} error={error}", path.display());
            None
        }
    }
}

/// The first of `file_names` that reads from `directory`.
fn read_first(directory: &std::path::Path, file_names: &[&str]) -> Option<Vec<u8>> {
    file_names
        .iter()
        .find_map(|file_name| std::fs::read(directory.join(file_name)).ok())
}
