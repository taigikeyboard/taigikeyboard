// The settings window's icon and VERSIONINFO — the `ProductName` /
// `ProductVersion` the updater pins a downloaded installer against.
// Logic shared with the DLL: `../../build-support/resource.rs`.

#[path = "../../build-support/resource.rs"]
mod resource;

fn main() {
    resource::embed(resource::Resources {
        file_description: "Taigi Keyboard settings",
        original_filename: "TaigiKeyboardSettings.exe",
        file_type: resource::FileType::Application,
        with_icon: true,
    });
}
