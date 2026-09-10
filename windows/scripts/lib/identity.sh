# Shared by the Windows release scripts: where things are and what version
# this checkout is. The version's single source of truth is
# `windows/Cargo.toml` `[workspace.package] version` (roadmap W8), which the
# repo-root `make version-desktop x.y.z` writes alongside macOS (the desktop
# train; iOS + Android are numbered separately).

fail() {
    echo "error: $*" >&2
    exit 1
}

WINDOWS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPOSITORY_DIR="$(cd "$WINDOWS_DIR/.." && pwd)"

APP_NAME="TaigiKeyboard"
# The VERSIONINFO ProductName the binaries and the installer must all declare
# (the updater compares them). Same string as the app name since the Windows
# surfaces were aligned with the macOS bundle name.
PRODUCT_NAME="$APP_NAME"
SERVICE_DLL="TaigiKeyboard.dll"
SETTINGS_EXE="TaigiKeyboardSettings.exe"
RELEASE_TARGET="x86_64-pc-windows-msvc"

# The first `version = "…"` after `[workspace.package]`.
SHORT_VERSION="$(awk '
    /^\[workspace\.package\]/ { inside = 1; next }
    inside && /^\[/ { exit }
    inside && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }
' "$WINDOWS_DIR/Cargo.toml")"

# The STRINGTABLE id every binary stores the localized product name under —
# the one the installer hands the shell for the Start-menu shortcut and the
# "Installed apps" entry. Read from the build script that WRITES the tables so
# the installer cannot drift from the binaries it ships.
PRODUCT_NAME_STRING_ID="$(awk '
    /^pub const PRODUCT_NAME_STRING_ID: u16 = / {
        gsub(/^pub const PRODUCT_NAME_STRING_ID: u16 = |;$/, ""); print; exit
    }
' "$WINDOWS_DIR/build-support/resource.rs")"
[[ "$PRODUCT_NAME_STRING_ID" =~ ^[0-9]+$ ]] ||
    fail "could not read PRODUCT_NAME_STRING_ID from windows/build-support/resource.rs"

DISTRIBUTION_DIR="$WINDOWS_DIR/.build/distribution"
STAGING_DIR="$WINDOWS_DIR/.build/staging"
TARGET_DIR="$WINDOWS_DIR/target/$RELEASE_TARGET/release"
DICTIONARIES_SOURCE_DIR="$REPOSITORY_DIR/dictionaries"
FONTS_SOURCE_DIR="$REPOSITORY_DIR/fonts/font"
INSTALLER_SCRIPT="$WINDOWS_DIR/installer/TaigiKeyboard.iss"
TASK_DEFINITION="$WINDOWS_DIR/installer/update-check-task.xml"

windows_path() {
    cygpath -w "$1" 2>/dev/null || printf '%s' "$1"
}

# Runs a native Windows tool with MSYS argument conversion switched off.
#
# Git Bash rewrites any argument that looks like an absolute POSIX path before
# a native binary sees it, and a Windows `/switch` is indistinguishable from a
# one-component path: `dumpbin /nologo` reached dumpbin as
# `C:\Program Files\Git\nologo`, which it opened as an input file and failed
# on with LNK1181 — silently, because a `$(...)` under `set -e` prints nothing
# (observed 2026-09-01, the first time this script ran on a real machine).
#
# Tools whose switches take a `-` need nothing and do not go through here.
# This is for the ones whose switches are documented only with `/`.
#
# CONTRACT: conversion is off for EVERY argument, so a caller must hand this
# any filesystem path in native Windows form (`windows_path`) — the conversion
# it would otherwise have got is gone too.
run_windows_tool() {
    MSYS2_ARG_CONV_EXCL='*' MSYS_NO_PATHCONV=1 "$@"
}

# What the updater pins a downloaded package against
# (taigi-windows-update::verify): the VERSIONINFO ProductName and
# ProductVersion, and the signer. Read back from the built files through
# PowerShell — Git Bash has no other reader, and the updater reads the same
# block through Win32.

# `ProductName<TAB>ProductVersion` of a PE file.
#
# Trimmed, because the two producers pad differently: rustc writes the string
# and stops, Inno Setup patches its stub's placeholder in place and pads to
# its width with spaces. The updater trims for the same reason
# (`taigi-windows-update::verify::product_name_from_versioninfo`).
version_info_of() {
    powershell.exe -NoProfile -NonInteractive -Command \
        "\$info = (Get-Item -LiteralPath '$(windows_path "$1")').VersionInfo; Write-Output (\$info.ProductName.Trim() + [char]9 + \$info.ProductVersion.Trim())" |
        tr -d '\r'
}

# The upper-case SHA-1 thumbprint of a PE file's Authenticode signer; empty
# when unsigned or not trusted.
signer_thumbprint_of() {
    powershell.exe -NoProfile -NonInteractive -Command \
        "\$signature = Get-AuthenticodeSignature -LiteralPath '$(windows_path "$1")'; if (\$signature.Status -eq 'Valid') { Write-Output \$signature.SignerCertificate.Thumbprint }" |
        tr -d '\r'
}

# Fails unless `file` declares this product at this checkout's version.
require_version_info() {
    local file="$1" info
    info="$(version_info_of "$file")"
    [[ "$info" == "$PRODUCT_NAME	$SHORT_VERSION" ]] ||
        fail "$(basename "$file") VERSIONINFO reads '${info//	/ \/ }', expected '$PRODUCT_NAME / $SHORT_VERSION' — the updater would refuse it"
}
