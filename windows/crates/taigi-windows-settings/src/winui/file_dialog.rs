//! The file dialogs, owned by the settings window. `rfd` wants a
//! window to be modal to, and Reactor hands out no HWND — so the owner
//! comes from `platform::dialog_owner()`, read at the moment the dialog
//! opens. That call happens inside a message handler on the UI thread,
//! whose active window IS the settings window; an unowned dialog would be
//! free to fall behind it.

use std::path::PathBuf;

fn dialog() -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new();
    match taigi_windows_platform::dialog_owner() {
        // An owner is better than none, but a dialog with no owner still
        // opens: never refuse the user's export because a handle was not
        // there to be read.
        Some(owner) => dialog.set_parent(&owner),
        None => {
            log::warn!("file_dialog.no_owner_window");
            dialog
        }
    }
}

/// A kind of file a settings page writes and reads: the filter's
/// label, the extension a save writes, and the extensions an open accepts.
pub struct FileKind {
    label: &'static str,
    saved: &'static str,
    opened: &'static [&'static str],
}

/// The custom dictionary as `roman,hanji` lines.
pub const CSV: FileKind = FileKind {
    label: "CSV",
    saved: "csv",
    opened: &["csv", "txt"],
};

/// A `.taigi` backup.
pub const BACKUP: FileKind = FileKind {
    label: "Taigi backup",
    saved: "taigi",
    opened: taigi_desktop_core::settings::learning_records::BACKUP_OPEN_EXTENSIONS,
};

/// Where to write a `kind` file, or `None` because the user cancelled.
pub fn save(kind: &FileKind, suggested_name: &str) -> Option<PathBuf> {
    dialog()
        .set_file_name(suggested_name)
        .add_filter(kind.label, &[kind.saved])
        .save_file()
}

/// Which `kind` file to read, or `None` because the user cancelled.
pub fn open(kind: &FileKind) -> Option<PathBuf> {
    dialog().add_filter(kind.label, kind.opened).pick_file()
}

/// Which font file to take into the library, or `None` because the user
/// cancelled. The filter is the library's own roster of extensions, so the
/// dialog cannot offer a file the import would refuse
/// (`taigi_desktop_storage::ALLOWED_EXTENSIONS`).
pub fn pick_font() -> Option<PathBuf> {
    dialog()
        .add_filter(
            "Fonts",
            taigi_desktop_storage::ALLOWED_EXTENSIONS.as_slice(),
        )
        .pick_file()
}
