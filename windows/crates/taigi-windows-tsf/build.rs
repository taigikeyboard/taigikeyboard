// The DLL's icon (the input-method profile's, the tray button's) and its
// VERSIONINFO. Logic shared with the settings exe: `../../build-support/resource.rs`.

#[path = "../../build-support/resource.rs"]
mod resource;

fn main() {
    resource::embed(resource::Resources {
        file_description: "Taigi Keyboard text service",
        original_filename: "TaigiKeyboard.dll",
        file_type: resource::FileType::Library,
        with_icon: true,
    });
}
