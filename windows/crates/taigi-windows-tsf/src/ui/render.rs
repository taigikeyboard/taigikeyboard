//! Direct2D + DirectWrite: the factories, the private font collection (the
//! four bundled faces from `<install>\Fonts`), text formats per
//! [`FontSpec`], the [`TextMeasurer`] the geometry models measure with, and
//! the per-window render target with device-loss recovery (khiin
//! `render_factory.rs`; roadmap W4, Codex F3).

use crate::com_out_buffer;
use crate::module::install_directory;
use crate::wide::{to_wide, to_wide_nul};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use taigi_windows_core::candidates::{FontSpec, TextMeasurer};
use taigi_windows_core::settings::{
    CandidateFontChoice, CandidateFontSelection, CustomFontId, InstalledFontId, SettingChoice,
};
use windows::core::{Interface, Result, BOOL, PCWSTR};
use windows::Win32::Foundation::{D2DERR_RECREATE_TARGET, HANDLE, HWND, WAIT_TIMEOUT};
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
    IDWriteFontCollection3, IDWriteFontSetBuilder1, IDWriteInlineObject, IDWriteTextFormat,
    IDWriteTextLayout, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_LINE_METRICS,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_METRICS,
    DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;
use windows::Win32::System::Threading::WaitForSingleObject;

/// The install directory's font folder — where the installer (PR10) copies the
/// repo-root `fonts/font/` (macOS `bundle-app.sh:177-207`).
pub const FONTS_DIR_NAME: &str = "Fonts";
/// The face the UI (index labels) and the `System` choice draw in on
/// Windows 11 — the same family the WinUI settings window renders in, so
/// the two windows of this input method do not disagree.
const SYSTEM_FONT_FAMILY: &str = "Segoe UI Variable Text";
/// What Windows 10 has instead: the variable family shipped with 11.
const LEGACY_SYSTEM_FONT_FAMILY: &str = "Segoe UI";
const LOCALE: &str = "zh-TW";

/// The collection a text format is created against, and the family it asks
/// for — for a bundled face, one the user added, one the OS has, or none.
///
/// A face that cannot be produced answers with the system family and no
/// collection, which is the honest fallback the bundled roster already had:
/// a window in the system font, never one drawing nothing.
impl RenderFactory {
    fn face_of(
        &self,
        selection: CandidateFontSelection,
    ) -> (Option<IDWriteFontCollection>, String) {
        match selection {
            CandidateFontSelection::BuiltIn(choice) => {
                let collection: Option<IDWriteFontCollection> = self
                    .private_fonts
                    .as_ref()
                    .filter(|fonts| fonts.loaded.contains(&choice))
                    .and_then(|fonts| fonts.collection.cast().ok());
                let family = collection
                    .as_ref()
                    .and(bundled_family_name(choice))
                    .unwrap_or(self.system_family)
                    .to_owned();
                (collection, family)
            }
            CandidateFontSelection::Custom(id) => {
                let loaded = self.custom_font.borrow();
                match loaded.as_ref().filter(|font| font.id == id) {
                    Some(font) => (font.collection.cast().ok(), font.family_name.clone()),
                    None => (None, self.system_family.to_owned()),
                }
            }
            CandidateFontSelection::Installed(id) => {
                // Against the system collection this process verified the
                // family in (`installed_font_id`), named explicitly rather
                // than left as `None`: the format then resolves the family
                // in the same collection the check ran against.
                let resolved = self.installed_font.borrow();
                let fonts = self.system_fonts.borrow();
                match (
                    resolved.as_ref().filter(|font| font.id == id),
                    fonts.as_ref(),
                ) {
                    (Some(font), Some(fonts)) => {
                        (fonts.collection.cast().ok(), font.family.clone())
                    }
                    _ => (None, self.system_family.to_owned()),
                }
            }
        }
    }

    /// The id `family` draws under while the OS has it, or `None` because it
    /// does not — or because the name is not one DirectWrite can be asked
    /// for (it comes out of `settings.json`, which anything can write).
    ///
    /// Called at the top of every candidate window, like `custom_font_id`. The
    /// system collection is re-fetched when DirectWrite says it expired — a
    /// font installed or removed since — and every text format made against
    /// the old one is dropped then, so a family the OS replaced under its own
    /// name is drawn from the new bytes and one it removed falls back rather
    /// than drawing from a collection that no longer has it. A new id per
    /// family AND per collection generation, for the same reason
    /// `CustomFontId` is per loaded resource. The custom face, if one was
    /// loaded, is let go: the selection is not it any more.
    pub fn installed_font_id(&self, family: &str) -> Option<InstalledFontId> {
        self.forget_custom_font();
        let id = self.resolve_installed_font(family);
        if id.is_none() {
            self.forget_installed_font();
        }
        id
    }

    fn resolve_installed_font(&self, family: &str) -> Option<InstalledFontId> {
        if self.refresh_system_fonts_if_expired() {
            // Unconditionally, not only when an installed family was resolved:
            // a selection that was already falling back has formats cached
            // against the collection that just went stale too.
            self.installed_font.borrow_mut().take();
            self.drop_font_caches();
        }
        // Verified against this very collection already: the usual answer.
        if let Some(resolved) = self.installed_font.borrow().as_ref() {
            if resolved.family == family {
                return Some(resolved.id);
            }
        }
        {
            let fonts = self.system_fonts.borrow();
            if !collection_has_family(&fonts.as_ref()?.collection, family) {
                return None;
            }
        }
        let id = InstalledFontId(self.next_installed_font_id.get().wrapping_add(1));
        self.next_installed_font_id.set(id.0);
        *self.installed_font.borrow_mut() = Some(ResolvedInstalledFont {
            id,
            family: family.to_owned(),
        });
        self.drop_font_caches();
        Some(id)
    }

    /// Lets go of the resolved installed family, so a text format made for it
    /// is not served again under a selection that moved elsewhere.
    pub fn forget_installed_font(&self) {
        if self.installed_font.borrow_mut().take().is_some() {
            self.drop_font_caches();
        }
    }

    /// Lets go of whichever non-bundled face was resolved — what a bundled
    /// selection does, so a host stops holding a custom file and stops
    /// serving formats made for a face that is not being drawn.
    pub fn forget_selected_fonts(&self) {
        self.forget_custom_font();
        self.forget_installed_font();
    }

    /// Makes sure `system_fonts` reflects what the OS has installed right now,
    /// answering whether it was (re)fetched — in which case anything made
    /// against the previous collection is stale.
    ///
    /// DirectWrite signals a collection's expiration event when the installed
    /// set changed (`IDWriteFontCollection3::GetExpirationEvent`); polling it
    /// with a zero timeout is what makes this cheap enough for every candidate
    /// window.
    fn refresh_system_fonts_if_expired(&self) -> bool {
        let is_current = self.system_fonts.borrow().as_ref().is_some_and(|fonts| {
            // SAFETY: the handle is owned by the live collection; a zero
            // timeout only reads its state. Only a timeout proves the event
            // has not fired — a failed wait re-fetches rather than trusting a
            // collection it could not check.
            let state = unsafe { WaitForSingleObject(fonts.expiration, 0) };
            state == WAIT_TIMEOUT
        });
        if is_current {
            return false;
        }
        *self.system_fonts.borrow_mut() = fetch_system_fonts(&self.dwrite);
        true
    }

    /// The id `file_name`'s typeface draws under, loading it if this process
    /// has not already — or `None` because the file is gone or is not a
    /// typeface this Windows can read.
    ///
    /// Called at the top of every candidate window, before anything is
    /// measured: the settings window may have added, replaced or deleted the
    /// file since the last one, and a host process is not restarted for that.
    /// Loading is skipped while the name AND the file's fingerprint are the
    /// ones already held, so the usual answer costs one `stat`.
    ///
    /// A new id retires every cached text format made from the old one —
    /// dropped here rather than left to be looked up, because an
    /// `IDWriteTextFormat` holds the collection it was made against and would
    /// keep drawing the previous bytes.
    pub fn custom_font_id(&self, file_name: &str) -> Option<CustomFontId> {
        self.forget_installed_font();
        let Some(path) = self.custom_font_path(file_name) else {
            self.forget_custom_font();
            return None;
        };
        let Some(fingerprint) = FileFingerprint::of(&path) else {
            // The file is gone. Let go of it before falling back, so the host
            // stops holding a collection over bytes nobody selects — which is
            // also what lets the settings window delete the file.
            self.forget_custom_font();
            return None;
        };
        if let Some(loaded) = self.custom_font.borrow().as_ref() {
            if loaded.file_name == file_name && loaded.fingerprint == fingerprint {
                return Some(loaded.id);
            }
        }
        if self
            .failed_custom_font
            .borrow()
            .as_ref()
            .is_some_and(|(failed, seen)| failed == file_name && *seen == fingerprint)
        {
            return None;
        }
        // Against THIS factory, which is what the collection stays valid
        // against (`font_file`'s module doc).
        let (collection, info) = match taigi_windows_platform::font_file::load(&self.dwrite, &path)
        {
            Ok(loaded) => loaded,
            Err(error) => {
                log::warn!("fonts.custom_not_loaded error={error}");
                *self.failed_custom_font.borrow_mut() = Some((file_name.to_owned(), fingerprint));
                self.forget_custom_font();
                return None;
            }
        };
        self.failed_custom_font.borrow_mut().take();
        let id = CustomFontId(self.next_custom_font_id.get().wrapping_add(1));
        self.next_custom_font_id.set(id.0);
        *self.custom_font.borrow_mut() = Some(LoadedCustomFont {
            id,
            file_name: file_name.to_owned(),
            fingerprint,
            collection,
            family_name: info.family_name,
        });
        self.drop_font_caches();
        Some(id)
    }

    /// Lets go of the loaded custom face, whatever the reason: its file is
    /// gone, it stopped being a typeface, or a bundled face was selected.
    ///
    /// Not merely tidy. A host that keeps the collection keeps the FILE it was
    /// loaded from open enough for Windows to refuse deleting it, so the
    /// settings window's removal would fail for as long as that host lived —
    /// even while it was drawing in something else entirely.
    pub fn forget_custom_font(&self) {
        if self.custom_font.borrow_mut().take().is_some() {
            self.drop_font_caches();
        }
    }

    /// Both caches, together: a format and its ellipsis sign are made from the
    /// same collection, and one outliving the other would draw a trimmed cell
    /// in two typefaces.
    fn drop_font_caches(&self) {
        self.formats.borrow_mut().clear();
        self.ellipsis.borrow_mut().clear();
    }

    /// Where a library file lives, as one path component under the user's font
    /// folder. The name comes out of `settings.json`, which anything can
    /// write, so it is never joined verbatim.
    ///
    /// The folder itself is resolved once — including its absence, which is
    /// what an AppContainer host with no `%APPDATA%` gets — because resolving
    /// it CREATES it, and this runs per candidate window.
    fn custom_font_path(&self, file_name: &str) -> Option<PathBuf> {
        let component = Path::new(file_name).file_name()?;
        if component != std::ffi::OsStr::new(file_name) {
            return None;
        }
        let mut cached = self.fonts_directory.borrow_mut();
        let directory = cached
            .get_or_insert_with(|| taigi_windows_storage::fonts_directory().ok())
            .as_ref()?;
        Some(directory.join(component))
    }
}

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

/// The factories one activation draws with, plus the caches built from them.
///
/// FIELD ORDER IS THE DROP ORDER. Rust drops a struct's fields in declaration
/// order, and the font collections, text formats and inline objects here are
/// only valid while `dwrite` lives: a custom collection released after its
/// factory faults inside DirectWrite when a layout resolves the family
/// (measured 2026-09-11 — `taigi_windows_platform::font_file`'s module doc).
/// The rule this project holds to is therefore that a DirectWrite object is
/// dropped before the factory that built it, and the factories are declared
/// LAST so that falls out of the type. Nothing DirectWrite- or
/// Direct2D-shaped may be added after them.
pub struct RenderFactory {
    /// The bundled faces that loaded. A choice whose file is missing draws
    /// in the system face (as macOS `CandidateFontChoice.font(named:)`
    /// degrades) — per face, not per folder.
    private_fonts: Option<PrivateFonts>,
    /// The typeface the user added, as this process last loaded it. One at a
    /// time — only the SELECTED custom face is ever loaded, so a library of
    /// twenty costs one collection, and the ids handed out here are what keeps
    /// two of them out of one cached text format.
    custom_font: RefCell<Option<LoadedCustomFont>>,
    /// The id the next loaded custom face gets. Counted per factory — which is
    /// the scope the caches it keys are in — and never derived from the file
    /// name: a file REPLACED under the same name must not reach the format
    /// cached for the bytes it replaced.
    next_custom_font_id: Cell<u32>,
    /// The library file that would not load, and what it looked like. Kept so
    /// a broken or unreadable file is parsed ONCE rather than on every
    /// candidate window for as long as it stays selected.
    failed_custom_font: RefCell<Option<(String, FileFingerprint)>>,
    /// The OS's font collection as last fetched, with the event that says it
    /// went stale. Fetched once at construction — which is also where the UI
    /// family is probed — and again whenever the event fires.
    system_fonts: RefCell<Option<SystemFonts>>,
    /// The installed family the selection last resolved to, and its id. One
    /// at a time, like `custom_font`.
    installed_font: RefCell<Option<ResolvedInstalledFont>>,
    /// The id the next resolved installed family gets — per factory, like
    /// `next_custom_font_id`, and bumped whenever the family OR the collection
    /// it was verified in changes.
    next_installed_font_id: Cell<u32>,
    /// The user's font folder, resolved once: asking again is two directory
    /// creations per candidate window for a path that cannot move under a
    /// running process.
    fonts_directory: RefCell<Option<Option<PathBuf>>>,
    /// The UI family this Windows really carries, resolved once.
    system_family: &'static str,
    formats: RefCell<HashMap<FormatKey, IDWriteTextFormat>>,
    ellipsis: RefCell<HashMap<FormatKey, IDWriteInlineObject>>,
    /// Last, and last dropped: the collections, formats and inline objects
    /// above were built from these.
    d2d: ID2D1Factory,
    dwrite: IDWriteFactory3,
}

/// The bundled fonts as one DirectWrite collection, plus which choices it
/// really holds.
struct PrivateFonts {
    collection: IDWriteFontCollection1,
    loaded: Vec<CandidateFontChoice>,
}

/// The OS's font collection and how DirectWrite says when it expired.
struct SystemFonts {
    collection: IDWriteFontCollection1,
    /// Signalled once the installed set changed. Owned by `collection`, never
    /// closed here.
    expiration: HANDLE,
}

/// The OS's collection as of now, with its expiration event — or `None`
/// because DirectWrite would not answer. `IDWriteFontCollection3` is required
/// rather than optional: every Windows this installs on has it
/// (`MinVersion=10.0.17763`, Inno), and a collection whose staleness cannot
/// be asked about would have to be re-fetched on every show.
fn fetch_system_fonts(dwrite: &IDWriteFactory3) -> Option<SystemFonts> {
    let collection = taigi_windows_platform::font_file::system_collection(dwrite, true)
        .map_err(|error| log::warn!("fonts.system_collection_unavailable error={error}"))
        .ok()?;
    let newer: IDWriteFontCollection3 = collection.cast().ok()?;
    // SAFETY: the collection is live; the handle it answers with is its own.
    let expiration = unsafe { newer.GetExpirationEvent() };
    if expiration.is_invalid() {
        return None;
    }
    Some(SystemFonts {
        collection,
        expiration,
    })
}

/// Whether `collection` has a family called `family`. A name with an embedded
/// NUL is not one DirectWrite can be asked for — it would be truncated into
/// some other, valid name — so it is refused before the call.
fn collection_has_family(collection: &IDWriteFontCollection1, family: &str) -> bool {
    if family.is_empty() || family.contains('\0') {
        return false;
    }
    let name = to_wide_nul(family);
    let mut index = 0_u32;
    let mut exists = BOOL::default();
    // SAFETY: a live NUL-terminated name and two live out-parameters on the
    // calling thread.
    unsafe { collection.FindFamilyName(PCWSTR(name.as_ptr()), &mut index, &mut exists) }.is_ok()
        && exists.as_bool()
}

/// An OS-installed family the selection resolved to.
struct ResolvedInstalledFont {
    id: InstalledFontId,
    /// The family a text format asks for. Out of `settings.json`: display it,
    /// never log it.
    family: String,
}

/// One typeface out of the user's library, loaded.
struct LoadedCustomFont {
    id: CustomFontId,
    /// The library file it came from, and what the file looked like when it
    /// was read. Both, because the id has to change when either does: a
    /// different file is a different typeface, and the same path with new
    /// bytes is too.
    file_name: String,
    fingerprint: FileFingerprint,
    collection: IDWriteFontCollection1,
    /// The family a text format asks for. Out of the file the user chose:
    /// display it, never log it.
    family_name: String,
}

/// What tells this process that a library file changed without asking it to
/// re-read every byte: the last write time and the length.
///
/// A detector, not proof — a replacement that preserved both would slip past.
/// Imports never overwrite (`taigi_windows_storage::copy_in` suffixes a name
/// already taken), so the only way to arrange that is by hand, in Explorer.
#[derive(Clone, Copy, PartialEq, Eq)]
struct FileFingerprint {
    modified_ms: i64,
    length: u64,
}

impl FileFingerprint {
    fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_millis() as i64);
        Some(Self {
            modified_ms,
            length: metadata.len(),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct FormatKey {
    selection: CandidateFontSelection,
    /// Size in hundredths of a DIP, so an `f32` can key a map.
    size_centi: u32,
    /// Text centred in its box (the index slot) rather than leading.
    centered: bool,
}

impl FormatKey {
    fn of(font: FontSpec, centered: bool) -> Self {
        Self {
            selection: font.selection,
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
        let system_fonts = fetch_system_fonts(&dwrite);
        let system_family = system_family(system_fonts.as_ref().map(|fonts| &fonts.collection));
        Ok(Self {
            private_fonts,
            custom_font: RefCell::new(None),
            next_custom_font_id: Cell::new(0),
            failed_custom_font: RefCell::new(None),
            system_fonts: RefCell::new(system_fonts),
            installed_font: RefCell::new(None),
            next_installed_font_id: Cell::new(0),
            fonts_directory: RefCell::new(None),
            system_family,
            formats: RefCell::new(HashMap::new()),
            ellipsis: RefCell::new(HashMap::new()),
            d2d,
            dwrite,
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
        let (collection, family_name) = self.face_of(font.selection);
        let family = to_wide_nul(&family_name);
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

/// The UI family this Windows really carries: the Windows 11 variable face
/// when the system collection has it, Windows 10's otherwise.
fn system_family(collection: Option<&IDWriteFontCollection1>) -> &'static str {
    match collection {
        Some(collection) if collection_has_family(collection, SYSTEM_FONT_FAMILY) => {
            SYSTEM_FONT_FAMILY
        }
        _ => {
            log::debug!("fonts.system_variable_absent");
            LEGACY_SYSTEM_FONT_FAMILY
        }
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
