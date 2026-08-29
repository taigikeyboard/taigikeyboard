// Shared by the two Windows crates' build scripts (`include!`d, not a crate:
// a build script cannot depend on a workspace member without a cycle).
//
// Writes the crate's Win32 resources — the icon and the VERSIONINFO block —
// and compiles them into an object the linker embeds. VERSIONINFO is what
// `ProductName` / `ProductVersion` are read from by the updater's package
// verification (`taigi-windows-update::verify`, roadmap W9): both the DLL
// and the settings exe carry the same `ProductName`, and the version is the
// workspace's.
//
// The resource compiler is whichever is on PATH: `rc.exe` (Windows SDK, the
// release build), `llvm-rc`, or GNU `windres` (the mingw cross-check on the
// macOS host). None found ⇒ a warning and no resources; the code paths that
// read them degrade (a stock icon in the tray, no in-app update install) —
// EXCEPT when `TAIGI_REQUIRE_RESOURCES=1` (the release script sets it): then a
// missing or failing compiler fails the build, and an MSVC target accepts no
// `windres` fallback (its COFF is not proven under `link.exe`).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const PRODUCT_NAME: &str = "Taigi Keyboard";
const COMPANY_NAME: &str = "Taigi Keyboard";
const COPYRIGHT: &str = "MIT License";
/// `lang_bar::ICON_RESOURCE_ID` and `RegisterProfile`'s icon index 0 both
/// name the first (only) icon resource.
const ICON_RESOURCE_ID: u16 = 1;

/// VERSIONINFO `FILETYPE`. Each build script that includes this file uses
/// one variant; the other is not dead, it is the other crate's.
#[allow(dead_code)]
pub enum FileType {
    /// `VFT_APP`.
    Application,
    /// `VFT_DLL`.
    Library,
}

pub struct Resources<'a> {
    pub file_description: &'a str,
    pub original_filename: &'a str,
    pub file_type: FileType,
    pub with_icon: bool,
}

pub fn embed(resources: Resources<'_>) {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let icon = manifest_dir.join("../../resources/TaigiKeyboard.ico");
    let shared = manifest_dir.join("../../build-support/resource.rs");
    println!("cargo:rerun-if-changed={}", icon.display());
    println!("cargo:rerun-if-changed={}", shared.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=TAIGI_REQUIRE_RESOURCES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let script = out_dir.join("resources.rc");
    std::fs::write(&script, render(&resources, &icon)).expect("write resources.rc");
    let required = env::var("TAIGI_REQUIRE_RESOURCES").as_deref() == Ok("1");
    match compile(&script, &out_dir, required) {
        Some(object) => println!("cargo:rustc-link-arg={}", object.display()),
        None if required => panic!(
            "TAIGI_REQUIRE_RESOURCES=1: no resource compiler (rc.exe / llvm-rc) compiled {}'s icon and VERSIONINFO — the updater's package check reads VERSIONINFO, so a release without it is refused",
            resources.original_filename
        ),
        None => println!(
            "cargo:warning=no resource compiler (rc.exe / llvm-rc / windres) on PATH: {} is built without its icon and VERSIONINFO",
            resources.original_filename
        ),
    }
}

fn render(resources: &Resources<'_>, icon: &Path) -> String {
    let version = env::var("CARGO_PKG_VERSION").expect("version");
    let mut parts = version
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let (major, minor, patch) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );
    let icon_line = if resources.with_icon {
        format!(
            "{ICON_RESOURCE_ID} ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "\\\\")
        )
    } else {
        String::new()
    };
    format!(
        r#"{icon_line}
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE {file_type}
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "{COMPANY_NAME}"
            VALUE "FileDescription", "{description}"
            VALUE "FileVersion", "{version}"
            VALUE "LegalCopyright", "{COPYRIGHT}"
            VALUE "OriginalFilename", "{original}"
            VALUE "ProductName", "{PRODUCT_NAME}"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#,
        description = resources.file_description,
        original = resources.original_filename,
        file_type = match resources.file_type {
            FileType::Application => "0x1L",
            FileType::Library => "0x2L",
        },
    )
}

/// The compiled resource object, or `None` when no compiler ran.
fn compile(script: &Path, out_dir: &Path, required: bool) -> Option<PathBuf> {
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    // An explicit `RC=<tool>` wins; then the SDK's rc.exe / llvm-rc (both
    // speak `/fo`), then GNU windres (the cross toolchain) — never windres
    // for a required MSVC build.
    let mut candidates: Vec<String> = env::var("RC").ok().into_iter().collect();
    if target_env == "msvc" {
        candidates.extend(["rc.exe", "rc", "llvm-rc"].map(str::to_owned));
    }
    if !(required && target_env == "msvc") {
        candidates.extend(["x86_64-w64-mingw32-windres", "windres"].map(str::to_owned));
    }
    candidates.extend(["llvm-rc", "rc.exe"].map(str::to_owned));
    for tool in candidates {
        let is_windres = tool.contains("windres");
        let output = if is_windres {
            out_dir.join("resources.o")
        } else {
            out_dir.join("resources.res")
        };
        let mut command = Command::new(&tool);
        if is_windres {
            command
                .arg("-O")
                .arg("coff")
                .arg("-i")
                .arg(script)
                .arg("-o")
                .arg(&output);
        } else {
            command
                .arg("/nologo")
                .arg(format!("/fo{}", output.display()))
                .arg(script);
        }
        match command.status() {
            Ok(status) if status.success() => return Some(output),
            Ok(status) => println!("cargo:warning={tool} failed with {status}"),
            Err(_) => continue,
        }
    }
    None
}
