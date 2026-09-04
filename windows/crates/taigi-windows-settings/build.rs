// The settings window's icon and VERSIONINFO — the `ProductName` /
// `ProductVersion` the updater pins a downloaded installer against.
// Logic shared with the DLL: `../../build-support/resource.rs`.

#[path = "../../build-support/resource.rs"]
mod resource;

fn main() {
    resource::embed(resource::Resources {
        file_description: "TaigiKeyboard settings",
        original_filename: "TaigiKeyboardSettings.exe",
        file_type: resource::FileType::Application,
        with_icon: true,
    });
    // W17: the Windows App Runtime the WinUI window runs on, staged beside
    // the exe (self-contained — no runtime install on the user's machine)
    // and its activatable-class manifest embedded by the linker. The MSVC
    // shipping target only: the setup crate refuses the gnu host check, and
    // the macOS-native test build has no Windows target at all.
    let is_msvc_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if is_msvc_windows {
        windows_reactor_setup::as_self_contained();
    }
}
