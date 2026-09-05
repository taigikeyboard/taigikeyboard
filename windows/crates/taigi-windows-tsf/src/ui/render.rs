//! Direct2D + DirectWrite: the factories, the private font collection (the
//! four bundled faces from `<install>\Fonts`), text formats per
//! [`FontSpec`], the [`TextMeasurer`] the geometry models measure with, and
//! the per-window render target with device-loss recovery (khiin
//! `render_factory.rs`; roadmap W4, Codex F3).

use crate::com_out_buffer;
use crate::module::install_directory;
use crate::wide::{to_wide, to_wide_nul};
use std::cell::RefCell;
use std::collections::HashMap;
use taigi_windows_core::candidates::{FontSpec, TextMeasurer};
use taigi_windows_core::settings::{CandidateFontChoice, SettingChoice};
use windows::core::{Interface, Result, BOOL, PCWSTR};
use windows::Win32::Foundation::{D2DERR_RECREATE_TARGET, HWND};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1SolidColorBrush,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory3, IDWriteFontCollection, IDWriteFontCollection1,
    IDWriteFontSetBuilder1, IDWriteInlineObject, IDWriteTextFormat, IDWriteTextLayout,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_LINE_METRICS, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_METRICS, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;

/// The install directory's font folder — where the installer (PR10) copies
/// `ios/Resources/Fonts/` (macOS `bundle-app.sh:178-201`).
pub const FONTS_DIR_NAME: &str = "Fonts";
/// The face the UI (index labels) and the `System` choice draw in on
/// Windows 11 — the same family the WinUI settings window renders in, so
/// the two windows of this input method do not disagree.
const SYSTEM_FONT_FAMILY: &str = "Segoe UI Variable Text";
/// What Windows 10 has instead: the variable family shipped with 11.
const LEGACY_SYSTEM_FONT_FAMILY: &str = "Segoe UI";
const LOCALE: &str = "zh-TW";

/// Family names as the bundled files declare them (what a text format asks
/// for). Mirrors macOS's PostScript names on the same files. `System` is
/// not one of them — it is whatever this Windows carries, which only
/// [`system_family`] knows.
fn bundled_family_name(choice: CandidateFontChoice) -> Option<&'static str> {
    Some(match choice {
        CandidateFontChoice::System => return None,
        CandidateFontChoice::OpenHuninn => "jf-openhuninn-2.1",
        CandidateFontChoice::Iansui => "Iansui",
        CandidateFontChoice::GenYoMin => "GenYoMin2TW",
        CandidateFontChoice::GenYoGothic => "GenYoGothic2TW",
    })
}

/// Process-wide factories plus the caches built from them.
pub struct RenderFactory {
    d2d: ID2D1Factory,
    dwrite: IDWriteFactory3,
    /// The bundled faces that loaded. A choice whose file is missing draws
    /// in the system face (as macOS `CandidateFontChoice.font(named:)`
    /// degrades) — per face, not per folder.
    private_fonts: Option<PrivateFonts>,
    /// The UI family this Windows really carries, resolved once.
    system_family: &'static str,
    formats: RefCell<HashMap<FormatKey, IDWriteTextFormat>>,
    ellipsis: RefCell<HashMap<FormatKey, IDWriteInlineObject>>,
}

/// The bundled fonts as one DirectWrite collection, plus which choices it
/// really holds.
struct PrivateFonts {
    collection: IDWriteFontCollection1,
    loaded: Vec<CandidateFontChoice>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FormatKey {
    choice: CandidateFontChoice,
    /// Size in hundredths of a DIP, so an `f32` can key a map.
    size_centi: u32,
    /// Text centred in its box (the index slot) rather than leading.
    centered: bool,
}

impl FormatKey {
    fn of(font: FontSpec, centered: bool) -> Self {
        Self {
            choice: font.choice,
            size_centi: (font.size * 100.0).round() as u32,
            centered,
        }
    }
}

impl RenderFactory {
    pub fn new() -> Result<Self> {
        // SAFETY: plain factory creation on the calling (UI) thread.
        let (d2d, dwrite): (ID2D1Factory, IDWriteFactory3) = unsafe {
            (
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?,
                DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?,
            )
        };
        let private_fonts = load_private_fonts(&dwrite);
        let system_family = system_family(&dwrite);
        Ok(Self {
            d2d,
            dwrite,
            private_fonts,
            system_family,
            formats: RefCell::new(HashMap::new()),
            ellipsis: RefCell::new(HashMap::new()),
        })
    }

    /// The text format for `font`, cached: no wrapping, vertically centred,
    /// so one layout per cell measures and draws alike.
    pub fn format(&self, font: FontSpec) -> Result<IDWriteTextFormat> {
        self.format_with(font, false)
    }

    fn format_with(&self, font: FontSpec, centered: bool) -> Result<IDWriteTextFormat> {
        let key = FormatKey::of(font, centered);
        if let Some(format) = self.formats.borrow().get(&key) {
            return Ok(format.clone());
        }
        let collection: Option<IDWriteFontCollection> = self
            .private_fonts
            .as_ref()
            .filter(|fonts| fonts.loaded.contains(&font.choice))
            .and_then(|fonts| fonts.collection.cast().ok());
        let family = to_wide_nul(
            collection
                .as_ref()
                .and(bundled_family_name(font.choice))
                .unwrap_or(self.system_family),
        );
        // SAFETY: valid NUL-terminated strings; the collection is ours or none.
        let format = unsafe {
            self.dwrite.CreateTextFormat(
                PCWSTR(family.as_ptr()),
                collection.as_ref(),
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font.size,
                PCWSTR(to_wide_nul(LOCALE).as_ptr()),
            )?
        };
        // SAFETY: configuration calls on the format just created.
        unsafe {
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            if centered {
                format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            }
        }
        self.formats.borrow_mut().insert(key, format.clone());
        Ok(format)
    }

    /// A layout of `text` in `font`, `max_width` wide (or unbounded for a
    /// measurement), trimmed with an ellipsis when it does not fit — the
    /// `byTruncatingTail` of the Mac labels.
    pub fn layout(
        &self,
        text: &str,
        font: FontSpec,
        max_width: f32,
        max_height: f32,
    ) -> Result<IDWriteTextLayout> {
        self.layout_with(text, font, max_width, max_height, false)
    }

    /// A layout centred in its box — the index slot.
    pub fn layout_centered(
        &self,
        text: &str,
        font: FontSpec,
        max_width: f32,
        max_height: f32,
    ) -> Result<IDWriteTextLayout> {
        self.layout_with(text, font, max_width, max_height, true)
    }

    fn layout_with(
        &self,
        text: &str,
        font: FontSpec,
        max_width: f32,
        max_height: f32,
        centered: bool,
    ) -> Result<IDWriteTextLayout> {
        let format = self.format_with(font, centered)?;
        let units = to_wide(text);
        // SAFETY: a layout over a slice that outlives the call (DWrite copies).
        let layout = unsafe {
            self.dwrite
                .CreateTextLayout(&units, &format, max_width, max_height)?
        };
        if max_width.is_finite() && max_width < f32::MAX {
            let key = FormatKey::of(font, centered);
            let sign = match self.ellipsis.borrow().get(&key) {
                Some(sign) => sign.clone(),
                // SAFETY: the sign is created from the same format.
                None => unsafe { self.dwrite.CreateEllipsisTrimmingSign(&format)? },
            };
            self.ellipsis.borrow_mut().insert(key, sign.clone());
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            // SAFETY: valid trimming struct + inline object.
            unsafe { layout.SetTrimming(&trimming, &sign)? };
        }
        Ok(layout)
    }

    /// A render target on `hwnd`, DPI-scaled so every coordinate is a DIP.
    pub fn hwnd_target(
        &self,
        hwnd: HWND,
        pixel_size: D2D_SIZE_U,
        dpi: f32,
    ) -> Result<ID2D1HwndRenderTarget> {
        let target_properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_UNKNOWN,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: dpi,
            dpiY: dpi,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let hwnd_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd,
            pixelSize: pixel_size,
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        // SAFETY: both property structs are valid locals.
        unsafe {
            self.d2d
                .CreateHwndRenderTarget(&target_properties, &hwnd_properties)
        }
    }
}

/// The UI family to draw in: the Windows 11 variable face when this system
/// carries it, the Windows 10 one otherwise.
///
/// Probed rather than left to DirectWrite's mapper: `CreateTextFormat`
/// accepts a family that does not exist and substitutes at LAYOUT time, so
/// a missing family would not fail anywhere this code could see — it would
/// just draw in whatever the mapper picked, and measure in it too.
fn system_family(dwrite: &IDWriteFactory3) -> &'static str {
    // SAFETY: a collection query and a name lookup on the calling thread;
    // the out-parameters are live locals.
    unsafe {
        let mut collection: Option<IDWriteFontCollection1> = None;
        if dwrite
            .GetSystemFontCollection(false, &mut collection, false)
            .is_err()
        {
            return LEGACY_SYSTEM_FONT_FAMILY;
        }
        let Some(collection) = collection else {
            return LEGACY_SYSTEM_FONT_FAMILY;
        };
        let name = to_wide_nul(SYSTEM_FONT_FAMILY);
        let mut index = 0u32;
        let mut exists = BOOL::default();
        if collection
            .FindFamilyName(PCWSTR(name.as_ptr()), &mut index, &mut exists)
            .is_err()
            || !exists.as_bool()
        {
            log::debug!("fonts.system_variable_absent");
            return LEGACY_SYSTEM_FONT_FAMILY;
        }
        SYSTEM_FONT_FAMILY
    }
}

/// The bundled faces that exist, as one collection. Missing folder or no
/// face at all → `None`.
fn load_private_fonts(dwrite: &IDWriteFactory3) -> Option<PrivateFonts> {
    let directory = install_directory()?.join(FONTS_DIR_NAME);
    // SAFETY: DWrite objects on the calling thread; every path is a live
    // NUL-terminated buffer for its call.
    unsafe {
        let builder: IDWriteFontSetBuilder1 = dwrite.CreateFontSetBuilder().ok()?.cast().ok()?;
        let mut loaded = Vec::new();
        for choice in CandidateFontChoice::ALL {
            let Some(file_name) = choice.file_name() else {
                continue;
            };
            let path = to_wide_nul(&directory.join(file_name).to_string_lossy());
            match dwrite
                .CreateFontFileReference(PCWSTR(path.as_ptr()), None)
                .and_then(|file| builder.AddFontFile(&file))
            {
                Ok(()) => loaded.push(*choice),
                Err(error) => log::warn!("fonts.missing file={file_name} error={error}"),
            }
        }
        if loaded.is_empty() {
            log::warn!("fonts.none_loaded directory={}", directory.display());
            return None;
        }
        let set = builder.CreateFontSet().ok()?;
        let collection = dwrite.CreateFontCollectionFromFontSet(&set).ok()?;
        Some(PrivateFonts { collection, loaded })
    }
}

/// The measurer the geometry models use: DirectWrite's own advances.
pub struct DWriteMeasurer<'a> {
    pub factory: &'a RenderFactory,
}

impl TextMeasurer for DWriteMeasurer<'_> {
    fn width(&self, text: &str, font: FontSpec) -> f32 {
        let Ok(layout) = self.factory.layout(text, font, f32::MAX, f32::MAX) else {
            return 0.0;
        };
        let mut metrics = DWRITE_TEXT_METRICS::default();
        // SAFETY: out-pointer to a local.
        if unsafe { layout.GetMetrics(&mut metrics) }.is_err() {
            return 0.0;
        }
        metrics.widthIncludingTrailingWhitespace.ceil()
    }

    fn line_height(&self, font: FontSpec) -> f32 {
        let Ok(layout) = self.factory.layout("永", font, f32::MAX, f32::MAX) else {
            return font.size * 1.3;
        };
        let mut lines = [DWRITE_LINE_METRICS::default()];
        let mut count = 0u32;
        // SAFETY: a one-line buffer plus its count out-pointer, through
        // `com_out_buffer` — the generated wrapper's pointer is read-only.
        let outcome = unsafe { com_out_buffer::line_metrics(&layout, &mut lines, &mut count) };
        if outcome.is_err() && count == 0 {
            return font.size * 1.3;
        }
        lines[0].height.ceil()
    }
}

/// One window's Direct2D surface: recreated whole on device loss
/// (`D2DERR_RECREATE_TARGET`), as Codex F3 asks for explicitly.
pub struct Surface {
    pub target: ID2D1HwndRenderTarget,
    pub brush: ID2D1SolidColorBrush,
}

impl Surface {
    pub fn create(
        factory: &RenderFactory,
        hwnd: HWND,
        pixel_size: D2D_SIZE_U,
        dpi: f32,
    ) -> Result<Self> {
        let target = factory.hwnd_target(hwnd, pixel_size, dpi)?;
        let white = D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        // SAFETY: a brush on the target just created.
        let brush = unsafe { target.CreateSolidColorBrush(&white, None)? };
        Ok(Self { target, brush })
    }

    /// Runs `draw` between `BeginDraw` / `EndDraw`. `Err(true)` = the
    /// device was lost and the surface must be recreated.
    pub fn frame(
        &self,
        draw: impl FnOnce(&ID2D1HwndRenderTarget, &ID2D1SolidColorBrush),
    ) -> std::result::Result<(), bool> {
        // SAFETY: the target is this surface's; Begin/End are balanced.
        unsafe {
            self.target.BeginDraw();
            draw(&self.target, &self.brush);
            match self.target.EndDraw(None, None) {
                Ok(()) => Ok(()),
                Err(error) => Err(error.code() == D2DERR_RECREATE_TARGET),
            }
        }
    }
}
