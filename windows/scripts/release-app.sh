#!/usr/bin/env bash
# Cut a Windows release (roadmap W8): a clean-tree preflight, the release
# builds, the staged install layout, signing (when a certificate is named),
# the Inno Setup installer, its signature, a SHA-256 — and, with --publish,
# the website repository's release + manifest. Mirrors
# macos/scripts/release-app.sh step for step; runs in Git Bash on a Windows
# machine that has: the Rust MSVC target, Inno Setup 6.5+ (`iscc` on PATH or
# in its default folder), the Windows SDK (`rc.exe`, `signtool`), Visual
# Studio Build Tools (`dumpbin`, beside `link.exe`), PowerShell, Python 3,
# and `gh`.
#
#   bash windows/scripts/release-app.sh [--allow-dirty] [--skip-sign] [--force] [--publish]
#
# Signing is by thumbprint: WINDOWS_SIGNING_THUMBPRINT names the Authenticode
# certificate in the current user's store; TIMESTAMP_URL the RFC 3161 server.
# Without a thumbprint the build is refused unless --skip-sign, and an
# unsigned build is never published: the updater pins the signer
# (docs/architecture/windows-release.md).

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/identity.sh"

TIMESTAMP_URL="${TIMESTAMP_URL:-http://timestamp.digicert.com}"
allow_dirty=false
skip_sign=false
force_overwrite=false
publish=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --allow-dirty) allow_dirty=true ;;
        --skip-sign) skip_sign=true ;;
        --force) force_overwrite=true ;;
        --publish) publish=true ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: release-app.sh [--allow-dirty] [--skip-sign] [--force] [--publish]" >&2
            exit 2
            ;;
    esac
    shift
done
if [[ "$publish" == true ]]; then
    [[ "$skip_sign" == false ]] ||
        fail "--publish and --skip-sign contradict: the updater refuses an unsigned installer"
    [[ "$allow_dirty" == false ]] ||
        fail "--publish and --allow-dirty contradict: a published installer must be reproducible from a commit"
fi

[[ "$SHORT_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
    fail "windows/Cargo.toml version '$SHORT_VERSION' is not MAJOR.MINOR.PATCH — run 'make version-desktop x.y.z' at the repository root"

echo "==> Checking the working tree"
TREE_STATUS="$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)"
if [[ -n "$TREE_STATUS" ]]; then
    if [[ "$allow_dirty" == false ]]; then
        echo "$TREE_STATUS" >&2
        fail "working tree is dirty — commit first, or pass --allow-dirty for a throwaway build"
    fi
    echo "  ⚠ dirty tree — this installer is a throwaway, do not publish it"
fi
HEAD_COMMIT="$(git -C "$REPOSITORY_DIR" rev-parse --short HEAD)"
QUALIFIER=""
[[ -z "$TREE_STATUS" ]] || QUALIFIER="$QUALIFIER-dirty"
[[ "$skip_sign" == false ]] || QUALIFIER="$QUALIFIER-unsigned"
OUTPUT_EXE="$DISTRIBUTION_DIR/$APP_NAME-$SHORT_VERSION$QUALIFIER-Setup.exe"
if [[ -e "$OUTPUT_EXE" && "$force_overwrite" == false ]]; then
    fail "$OUTPUT_EXE already exists — bump the version, or pass --force"
fi

echo "==> Checking the tools"
for tool in cargo cygpath sha256sum base64 curl python3 powershell.exe; do
    command -v "$tool" > /dev/null || fail "$tool is not on PATH (Git Bash, PowerShell and Python 3 are prerequisites)"
done
rustup target list --installed 2>/dev/null | grep -qx "$RELEASE_TARGET" ||
    fail "the $RELEASE_TARGET target is not installed (rustup target add $RELEASE_TARGET)"
resolve_iscc() {
    if command -v iscc > /dev/null; then
        command -v iscc
        return
    fi
    local candidate
    for candidate in "${ISCC:-}" "/c/Program Files (x86)/Inno Setup 6/ISCC.exe" "/c/Program Files/Inno Setup 6/ISCC.exe"; do
        [[ -n "$candidate" && -x "$candidate" ]] && { printf '%s' "$candidate"; return; }
    done
    fail "Inno Setup 6.5+ (ISCC.exe) not found — install it or set ISCC=<path>"
}
ISCC_BIN="$(resolve_iscc)"
command -v rc.exe > /dev/null || command -v rc > /dev/null ||
    fail "rc.exe (Windows SDK) is not on PATH — run from a Developer Command Prompt, or add the SDK's bin to PATH; without it the DLL and exe carry no icon or VERSIONINFO"
command -v dumpbin > /dev/null ||
    fail "dumpbin (Visual Studio Build Tools, beside link.exe) is not on PATH — the import-table check needs it"
SIGN_THUMBPRINT="${WINDOWS_SIGNING_THUMBPRINT:-}"
if [[ "$skip_sign" == false ]]; then
    [[ -n "$SIGN_THUMBPRINT" ]] ||
        fail "WINDOWS_SIGNING_THUMBPRINT is not set — see docs/architecture/windows-release.md, or pass --skip-sign for a throwaway build"
    command -v signtool > /dev/null || fail "signtool (Windows SDK) is not on PATH"
fi
if [[ "$publish" == true ]]; then
    command -v gh > /dev/null || fail "the GitHub CLI (gh) is not installed"
    gh auth status > /dev/null 2>&1 || fail "gh is not authenticated — run 'gh auth login'"
fi

sign_file() {
    local file="$1"
    [[ "$skip_sign" == false ]] || return 0
    signtool sign /fd SHA256 /td SHA256 /tr "$TIMESTAMP_URL" /sha1 "$SIGN_THUMBPRINT" "$file" > /dev/null ||
        fail "signing $file failed"
    signtool verify /pa /q "$file" > /dev/null || fail "$file does not verify after signing"
}

echo "==> Building the release binaries ($RELEASE_TARGET)"
# TAIGI_REQUIRE_RESOURCES: a resource-compile failure FAILS the build instead
# of warning (build-support/resource.rs) — the updater reads VERSIONINFO.
(cd "$WINDOWS_DIR" && TAIGI_REQUIRE_RESOURCES=1 cargo build --release --target "$RELEASE_TARGET" -p taigi-windows-tsf -p taigi-windows-settings)
[[ -f "$TARGET_DIR/$SERVICE_DLL" ]] || fail "no $SERVICE_DLL in $TARGET_DIR"
[[ -f "$TARGET_DIR/$SETTINGS_EXE" ]] || fail "no $SETTINGS_EXE in $TARGET_DIR"

echo "==> Reading back what the binaries declare"
# Proved from the files, not assumed from the compiler directives: the
# VERSIONINFO the updater pins (taigi-windows-update::verify), and — W14,
# `+crt-static` — no VC runtime import, or the build fails on a machine
# without the redistributable.
require_version_info "$TARGET_DIR/$SERVICE_DLL"
require_version_info "$TARGET_DIR/$SETTINGS_EXE"
for binary in "$TARGET_DIR/$SERVICE_DLL" "$TARGET_DIR/$SETTINGS_EXE"; do
    imports="$(dumpbin /nologo /dependents "$(windows_path "$binary")" | tr -d '\r')"
    if grep -iqE 'vcruntime[0-9]*(d)?\.dll|msvcp[0-9]*(d)?\.dll' <<< "$imports"; then
        echo "$imports" >&2
        fail "$(basename "$binary") imports the VC runtime — the release must be statically linked (+crt-static)"
    fi
done
# W17: the text service is loaded into every host process and must never
# pull WinUI / the Windows App Runtime in with it — only the settings exe
# links them.
dll_imports="$(dumpbin /nologo /dependents "$(windows_path "$TARGET_DIR/$SERVICE_DLL")" | tr -d '\r')"
if grep -iqE 'microsoft\.ui\.|windowsappruntime|microsoft\.internal\.frameworkudk' <<< "$dll_imports"; then
    echo "$dll_imports" >&2
    fail "$SERVICE_DLL imports WinUI / the Windows App Runtime — the text service must not (roadmap W17)"
fi
# WinUI is reached through activatable classes named in an embedded manifest,
# not through static imports (so the check above cannot see it): the exe must
# carry the setup crate's self-contained manifest, and the DLL must not.
WINUI_MANIFEST_MARKER="windows-reactor-self-contained"
grep -aq "$WINUI_MANIFEST_MARKER" "$TARGET_DIR/$SETTINGS_EXE" ||
    fail "$SETTINGS_EXE carries no self-contained Windows App Runtime manifest ($WINUI_MANIFEST_MARKER) — build.rs did not stage the runtime"
if grep -aq "$WINUI_MANIFEST_MARKER" "$TARGET_DIR/$SERVICE_DLL"; then
    fail "$SERVICE_DLL carries the WinUI manifest — the text service must not (roadmap W17)"
fi

echo "==> Staging the install layout"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/Dictionaries" "$STAGING_DIR/Fonts" "$DISTRIBUTION_DIR"
cp "$TARGET_DIR/$SERVICE_DLL" "$TARGET_DIR/$SETTINGS_EXE" "$STAGING_DIR/"
# The four dictionary artefacts, from the iOS resource directory (W2: no
# third committed copy); an empty one would be an engine with no words.
for artifact in dictionary.fst dictionary.bin association.bin syllables.fst; do
    source_file="$DICTIONARIES_SOURCE_DIR/$artifact"
    [[ -s "$source_file" ]] || fail "missing or empty dictionary artifact $source_file (run 'make dict' + 'make build')"
    cp "$source_file" "$STAGING_DIR/Dictionaries/"
done
# Every face in the directory — which faces exist is CandidateFontChoice's
# to state (macos/scripts/bundle-app.sh:178-201).
font_count=0
for candidate in "$FONTS_SOURCE_DIR"/*.ttf "$FONTS_SOURCE_DIR"/*.otf; do
    [[ -s "$candidate" ]] || continue
    cp "$candidate" "$STAGING_DIR/Fonts/"
    font_count=$((font_count + 1))
done
[[ $font_count -gt 0 ]] || fail "no font files in $FONTS_SOURCE_DIR"
cp "$TASK_DEFINITION" "$STAGING_DIR/"
# W17: the Windows App Runtime the settings exe runs on, staged beside the
# exe by its build script (self-contained; the names are the setup crate's
# list, vendored). Every entry must be there — a missing DLL is a window
# that will not open on the user's machine.
RUNTIME_LIST="$WINDOWS_DIR/build-support/windows-app-runtime-files.txt"
mkdir -p "$STAGING_DIR/Runtime"
runtime_count=0
while IFS= read -r name; do
    name="${name%%$'\r'}"
    [[ -z "$name" || "$name" == \#* ]] && continue
    source_path="$TARGET_DIR/$name"
    [[ -e "$source_path" ]] || fail "missing Windows App Runtime file $name in $TARGET_DIR (windows-reactor-setup staging)"
    cp -R "$source_path" "$STAGING_DIR/Runtime/"
    runtime_count=$((runtime_count + 1))
done < "$RUNTIME_LIST"
[[ $runtime_count -gt 0 ]] || fail "no Windows App Runtime entries in $RUNTIME_LIST"

echo "==> Signing the binaries"
sign_file "$STAGING_DIR/$SERVICE_DLL"
sign_file "$STAGING_DIR/$SETTINGS_EXE"

echo "==> Compiling the installer"
STAGING_WIN="$(windows_path "$STAGING_DIR")"
OUTPUT_WIN="$(windows_path "$DISTRIBUTION_DIR")"
"$ISCC_BIN" /Q "/DAppVersion=$SHORT_VERSION" "/DDist=$STAGING_WIN" "/O$OUTPUT_WIN" "$(windows_path "$INSTALLER_SCRIPT")" ||
    fail "iscc failed"
BUILT_EXE="$DISTRIBUTION_DIR/$APP_NAME-$SHORT_VERSION-Setup.exe"
[[ -f "$BUILT_EXE" ]] || fail "iscc produced no $BUILT_EXE"
# The installer's own VERSIONINFO (the .iss VersionInfo* directives) is what
# an installed copy verifies a downloaded package by.
require_version_info "$BUILT_EXE"

echo "==> Signing the installer"
sign_file "$BUILT_EXE"

if [[ "$BUILT_EXE" != "$OUTPUT_EXE" ]]; then
    mv -f "$BUILT_EXE" "$OUTPUT_EXE"
fi
echo ""
echo "✓ $OUTPUT_EXE"
echo "  version   $SHORT_VERSION"
echo "  commit    $HEAD_COMMIT"
echo "  installs  %ProgramFiles%\\TaigiKeyboard (administrator prompt)"
echo "  sha256    $(sha256sum "$OUTPUT_EXE" | cut -d' ' -f1)"
if [[ "$skip_sign" == true ]]; then
    echo ""
    echo "  ⚠ unsigned — SmartScreen warns on every machine, and the in-app updater will never install it."
fi

if [[ "$publish" == true ]]; then
    [[ "$(git -C "$REPOSITORY_DIR" rev-parse --short HEAD)" == "$HEAD_COMMIT" &&
        -z "$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)" ]] ||
        fail "the working tree changed during the build — this installer no longer matches $HEAD_COMMIT, rebuild before publishing"
    echo ""
    bash "$WINDOWS_DIR/scripts/publish-release.sh" --installer "$OUTPUT_EXE"
else
    echo ""
    echo "  Publish it with: bash windows/scripts/publish-release.sh"
fi
