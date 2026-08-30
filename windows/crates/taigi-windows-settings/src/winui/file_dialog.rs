//! The CSV file dialogs, owned by the settings window. `rfd` wants a
//! window to be modal to, and Reactor hands out no HWND — so the owner
//! comes from `platform::dialog_owner()`, read at the moment the dialog
//! opens. That call happens inside a message handler on the UI thread,
//! whose active window IS the settings window; an unowned dialog would be
//! free to fall behind it.

// 中文: CSV 檔案對話框。Reactor 不給 HWND,所以開對話框當下讀執行緒的 active window 當 owner(否則對話框會掉到視窗後面)。

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

/// Where to write a CSV, or `None` because the user cancelled.
pub fn save(suggested_name: &str) -> Option<PathBuf> {
    dialog()
        .set_file_name(suggested_name)
        .add_filter("CSV", &["csv"])
        .save_file()
}

/// Which CSV to read, or `None` because the user cancelled.
pub fn open() -> Option<PathBuf> {
    dialog().add_filter("CSV", &["csv", "txt"]).pick_file()
}
