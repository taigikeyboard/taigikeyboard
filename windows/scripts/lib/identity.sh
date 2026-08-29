# Shared by the Windows release scripts: where things are and what version
# this checkout is. The version's single source of truth is
# `windows/Cargo.toml` `[workspace.package] version` (roadmap W8), which the
# repo-root `make version x.y.z` writes alongside the other platforms.

fail() {
    echo "error: $*" >&2
    exit 1
}

WINDOWS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPOSITORY_DIR="$(cd "$WINDOWS_DIR/.." && pwd)"

APP_NAME="TaigiKeyboard"
PRODUCT_NAME="Taigi Keyboard"
SERVICE_DLL="TaigiKeyboard.dll"
SETTINGS_EXE="TaigiKeyboardSettings.exe"
RELEASE_TARGET="x86_64-pc-windows-msvc"

# The first `version = "…"` after `[workspace.package]`.
SHORT_VERSION="$(awk '
    /^\[workspace\.package\]/ { inside = 1; next }
    inside && /^\[/ { exit }
    inside && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }
' "$WINDOWS_DIR/Cargo.toml")"

DISTRIBUTION_DIR="$WINDOWS_DIR/.build/distribution"
STAGING_DIR="$WINDOWS_DIR/.build/staging"
TARGET_DIR="$WINDOWS_DIR/target/$RELEASE_TARGET/release"
DICTIONARIES_SOURCE_DIR="$REPOSITORY_DIR/ios/Resources/Dictionaries"
FONTS_SOURCE_DIR="$REPOSITORY_DIR/ios/Resources/Fonts"
INSTALLER_SCRIPT="$WINDOWS_DIR/installer/TaigiKeyboard.iss"
TASK_DEFINITION="$WINDOWS_DIR/installer/update-check-task.xml"

windows_path() {
    cygpath -w "$1" 2>/dev/null || printf '%s' "$1"
}

# What the updater pins a downloaded package against
# (taigi-windows-update::verify): the VERSIONINFO ProductName and
# ProductVersion, and the signer. Read back from the built files through
# PowerShell — Git Bash has no other reader, and the updater reads the same
# block through Win32.

# `ProductName<TAB>ProductVersion` of a PE file.
version_info_of() {
    powershell.exe -NoProfile -NonInteractive -Command \
        "\$info = (Get-Item -LiteralPath '$(windows_path "$1")').VersionInfo; Write-Output (\$info.ProductName + [char]9 + \$info.ProductVersion)" |
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
