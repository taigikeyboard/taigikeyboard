#!/usr/bin/env bash
# Cut a Windows release (roadmap W8): a clean-tree preflight, the release
# builds, the staged install layout, signing (when a certificate is named),
# the Inno Setup installer, its signature, a SHA-256 — and, with --publish,
# the website repository's release + manifest. Mirrors
# macos/scripts/release-app.sh step for step; runs in Git Bash on a Windows
# machine that has: the Rust MSVC targets (x64, i686 and aarch64), Inno Setup 6.5+
# (`iscc` on PATH or in its default folder), the Windows SDK (`rc.exe`, `signtool`), Visual
# Studio Build Tools with the "MSVC ARM64/ARM64EC build tools" component
# (`dumpbin`, beside `link.exe`; the Arm64X forwarder), PowerShell, Python 3,
# and `gh`.
#
#   bash windows/scripts/release-app.sh [--allow-dirty] [--skip-sign] [--force] [--publish]
#
# Signing is by thumbprint: WINDOWS_SIGNING_THUMBPRINT names the Authenticode
# certificate in the current user's store; TIMESTAMP_URL the RFC 3161 server.
# Without a thumbprint the build is refused unless --skip-sign.
#
# UNSIGNED IS A RELEASE CHANNEL, not a throwaway: `--skip-sign --publish`
# ships, and the installer carries the same plain name a signed one would.
# Why, what it costs the user, and when it ends are one fact in one place —
# docs/architecture/windows-release.md § Signing status.
#
# NAMED DIVERGENCE from macOS: macos/scripts/release-app.sh keeps its
# --publish x --skip-notarize contradiction and its `-unnotarized` qualifier,
# because macOS HAS a Developer ID certificate. Windows deliberately no longer
# mirrors that half.

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/identity.sh"
# shellcheck source=../../tools/release/lib/desktop-release.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../tools/release/lib/desktop-release.sh"

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
# Only what makes an installer UNPUBLISHABLE is spelled out in its name, so a
# throwaway can never be mistaken for the file people are told to download. An
# unsigned installer is publishable, so it carries the plain release name —
# the same one macOS publishes (`TaigiKeyboard-<version>.pkg`).
QUALIFIER=""
[[ -z "$TREE_STATUS" ]] || QUALIFIER="$QUALIFIER-dirty"
OUTPUT_EXE="$DISTRIBUTION_DIR/${WINDOWS_ASSET%.exe}$QUALIFIER.exe"
if [[ -e "$OUTPUT_EXE" && "$force_overwrite" == false ]]; then
    fail "$OUTPUT_EXE already exists — bump the version, or pass --force"
fi

echo "==> Checking the tools"
# What building needs. `base64`, `curl` and `python3` are checked further down
# instead: nothing in the build reads them, they belong to publish-release.sh,
# and requiring them here would refuse a build-only run on a machine that can
# perfectly well produce the installer.
for tool in cargo cygpath sha256sum powershell.exe; do
    command -v "$tool" > /dev/null || fail "$tool is not on PATH (Git Bash and PowerShell are prerequisites)"
done
# From windows/, so rustup answers for the toolchain rust-toolchain.toml pins
# there — the one cargo builds with — not for the caller's default.
installed_targets="$(cd "$WINDOWS_DIR" && rustup target list --installed 2>/dev/null)"
for target in "$RELEASE_TARGET" "$RELEASE_TARGET_32" "$RELEASE_TARGET_ARM64"; do
    grep -qx "$target" <<< "$installed_targets" ||
        fail "the $target target is not installed (rustup target add $target)"
done
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
        fail "WINDOWS_SIGNING_THUMBPRINT is not set — see docs/architecture/windows-release.md, or pass --skip-sign for the unsigned release channel"
    command -v signtool > /dev/null || fail "signtool (Windows SDK) is not on PATH"
fi
if [[ "$publish" == true ]]; then
    # Checked before the build rather than after it: publish-release.sh checks
    # the same list, but reaching it costs a full release build first.
    for tool in base64 curl python3; do
        command -v "$tool" > /dev/null || fail "$tool is not on PATH — publishing needs it (Python 3 is a prerequisite)"
    done
    command -v gh > /dev/null || fail "the GitHub CLI (gh) is not installed"
    gh auth status > /dev/null 2>&1 || fail "gh is not authenticated — run 'gh auth login'"
fi

# signtool's switches exist only in `/` form, so it runs through
# `run_windows_tool` — which means the file it signs must be handed over
# already converted, since it was the MSYS conversion that used to do that.
sign_file() {
    local file="$1" windows_file
    [[ "$skip_sign" == false ]] || return 0
    windows_file="$(windows_path "$file")"
    run_windows_tool signtool sign /fd SHA256 /td SHA256 /tr "$TIMESTAMP_URL" /sha1 "$SIGN_THUMBPRINT" "$windows_file" > /dev/null ||
        fail "signing $file failed"
    run_windows_tool signtool verify /pa /q "$windows_file" > /dev/null || fail "$file does not verify after signing"
}

# windows-reactor-setup extracts the Windows App Runtime once per machine,
# not per architecture (`ensure_msix_extracted` at the pinned windows-rs rev),
# so an Arm64 settings build after an x64 one would stage x64 runtime DLLs.
# Clearing the extraction before each architecture's build only spares a
# rebuild; what PROVES the staged runtime is the PE-machine read-back in
# `stage_runtime` — a build script cargo did not rerun leaves whatever an
# earlier run put in the target directory, and no cache clear reaches that.
clear_runtime_extraction() {
    local cache
    cache="$(cygpath -u "$LOCALAPPDATA")/windows-reactor-setup/temp"
    rm -rf "$cache"/*/.msix_extract
}

echo "==> Building the release binaries ($RELEASE_TARGET)"
# TAIGI_REQUIRE_RESOURCES: a resource-compile failure FAILS the build instead
# of warning (build-support/resource.rs) — the updater reads VERSIONINFO.
clear_runtime_extraction
(cd "$WINDOWS_DIR" && TAIGI_REQUIRE_RESOURCES=1 cargo build --release --locked --target "$RELEASE_TARGET" -p taigi-windows-tsf -p taigi-windows-settings)
[[ -f "$TARGET_DIR/$SERVICE_DLL" ]] || fail "no $SERVICE_DLL in $TARGET_DIR"
[[ -f "$TARGET_DIR/$SETTINGS_EXE" ]] || fail "no $SETTINGS_EXE in $TARGET_DIR"

echo "==> Building the 32-bit text service ($RELEASE_TARGET_32)"
# The DLL only: a 32-bit host loads the text service, never the settings
# window, which the 64-bit exe beside it serves for both.
(cd "$WINDOWS_DIR" && TAIGI_REQUIRE_RESOURCES=1 cargo build --release --locked --target "$RELEASE_TARGET_32" -p taigi-windows-tsf)
[[ -f "$TARGET_DIR_32/$SERVICE_DLL" ]] || fail "no $SERVICE_DLL in $TARGET_DIR_32"

echo "==> Building the Arm64 text service and settings window ($RELEASE_TARGET_ARM64)"
# Native Arm64 settings exe (roadmap D2); the installer picks it on an Arm64
# machine under the same install name.
clear_runtime_extraction
(cd "$WINDOWS_DIR" && TAIGI_REQUIRE_RESOURCES=1 cargo build --release --locked --target "$RELEASE_TARGET_ARM64" -p taigi-windows-tsf -p taigi-windows-settings)
[[ -f "$TARGET_DIR_ARM64/$SERVICE_DLL" ]] || fail "no $SERVICE_DLL in $TARGET_DIR_ARM64"
[[ -f "$TARGET_DIR_ARM64/$SETTINGS_EXE" ]] || fail "no $SETTINGS_EXE in $TARGET_DIR_ARM64"

echo "==> Linking the Arm64X forwarder"
# Its own PowerShell process: the script enters a Visual Studio developer
# environment for an Arm64 target. It fails unless the result is Arm64X.
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
    -File "$(windows_path "$WINDOWS_DIR/scripts/build-forwarder.ps1")" \
    -OutDir "$(windows_path "$FORWARDER_DIR")" ||
    fail "build-forwarder.ps1 failed"
FORWARDER="$FORWARDER_DIR/$FORWARDER_DLL"
[[ -f "$FORWARDER" ]] || fail "no $FORWARDER_DLL in $FORWARDER_DIR"

# dumpbin's report on `file` for one switch (`-headers`, `-exports`,
# `-dependents`), CR-stripped. Callers grep the captured text: piping dumpbin
# straight into `grep -q` under pipefail fails at random when grep exits early
# and the writer takes SIGPIPE.
dumpbin_read() {
    dumpbin -nologo "$1" "$(windows_path "$2")" | tr -d '\r'
}

echo "==> Reading back what the binaries declare"
# Proved from the files, not assumed from the compiler directives: the
# VERSIONINFO the updater pins (taigi-windows-update::verify), and — W14,
# `+crt-static` — no VC runtime import, or the build fails on a machine
# without the redistributable.
SERVICE_DLLS=("$TARGET_DIR/$SERVICE_DLL" "$TARGET_DIR_32/$SERVICE_DLL" "$TARGET_DIR_ARM64/$SERVICE_DLL")
SETTINGS_EXES=("$TARGET_DIR/$SETTINGS_EXE" "$TARGET_DIR_ARM64/$SETTINGS_EXE")
for binary in "${SERVICE_DLLS[@]}" "${SETTINGS_EXES[@]}"; do
    require_version_info "$binary"
    imports="$(dumpbin_read -dependents "$binary")"
    if grep -iqE 'vcruntime[0-9]*(d)?\.dll|msvcp[0-9]*(d)?\.dll|msvcr[0-9]*(d)?\.dll|msvcrt\.dll|ucrtbase(d)?\.dll|api-ms-win-crt-' <<< "$imports"; then
        echo "$imports" >&2
        fail "$(basename "$binary") imports the C runtime — the release must be statically linked (+crt-static)"
    fi
    # W17: the text service is loaded into every host process and must never
    # pull WinUI / the Windows App Runtime in with it — only the settings exe
    # links them.
    if [[ "$binary" != *"/$SETTINGS_EXE" ]] &&
        grep -iqE 'microsoft\.ui\.|windowsappruntime|microsoft\.internal\.frameworkudk' <<< "$imports"; then
        echo "$imports" >&2
        fail "$binary imports WinUI / the Windows App Runtime — the text service must not (roadmap W17)"
    fi
done
# WinUI is reached through activatable classes named in an embedded manifest,
# not through static imports (so the check above cannot see it): the exe must
# carry the setup crate's self-contained manifest, and the DLL must not.
WINUI_MANIFEST_MARKER="windows-reactor-self-contained"
for exe in "${SETTINGS_EXES[@]}"; do
    grep -aq "$WINUI_MANIFEST_MARKER" "$exe" ||
        fail "$exe carries no self-contained Windows App Runtime manifest ($WINUI_MANIFEST_MARKER) — build.rs did not stage the runtime"
done
# The Arm64X forwarder (roadmap § Forwarder): the product's VERSIONINFO, no
# imports at all (it is `/noentry`, two forwarded exports and nothing else),
# the Arm64X marker, and both views' forwards — the native view as dumpbin
# lists it, the x64 view's as the strings it holds, which dumpbin does not
# list. The installer-arm64 CI job proves the x64 view activates.
require_version_info "$FORWARDER"
forwarder_imports="$(dumpbin_read -dependents "$FORWARDER")"
if grep -qE 'has the following( delay load)? dependencies' <<< "$forwarder_imports"; then
    echo "$forwarder_imports" >&2
    fail "$FORWARDER_DLL imports a DLL — a pure forwarder imports nothing"
fi
forwarder_headers="$(dumpbin_read -headers "$FORWARDER")"
grep -qF 'AA64 machine (ARM64) (ARM64X)' <<< "$forwarder_headers" ||
    fail "$FORWARDER_DLL is not an Arm64X image"
forwarder_exports="$(dumpbin_read -exports "$FORWARDER")"
arm64_module="${SERVICE_DLL_ARM64%.dll}"
x64_module="${SERVICE_DLL%.dll}"
for symbol in DllGetClassObject DllCanUnloadNow; do
    grep -qF "$symbol (forwarded to $arm64_module.$symbol)" <<< "$forwarder_exports" ||
        fail "$FORWARDER_DLL does not forward $symbol to $SERVICE_DLL_ARM64 (Arm64 view)"
    grep -aqF "$x64_module.$symbol" "$FORWARDER" ||
        fail "$FORWARDER_DLL does not forward $symbol to $SERVICE_DLL (x64 view)"
done
for dll in "${SERVICE_DLLS[@]}"; do
    if grep -aq "$WINUI_MANIFEST_MARKER" "$dll"; then
        fail "$dll carries the WinUI manifest — the text service must not (roadmap W17)"
    fi
    # The four entry points regsvr32 looks for: an export table that lost one
    # registers nothing, and the failure surfaces on the user's machine. The
    # 32-bit exports are undecorated too (measured 2026-09-01), so one pattern
    # reads both.
    exports="$(dumpbin_read -exports "$dll")"
    for symbol in DllGetClassObject DllCanUnloadNow DllRegisterServer DllUnregisterServer; do
        grep -qE "[[:space:]]$symbol([[:space:]]|=|\$)" <<< "$exports" ||
            fail "$dll does not export $symbol"
    done
done

echo "==> Staging the install layout"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/Dictionaries" "$STAGING_DIR/Fonts" "$STAGING_DIR/arm64" "$DISTRIBUTION_DIR"
cp "$TARGET_DIR/$SERVICE_DLL" "$TARGET_DIR/$SETTINGS_EXE" "$STAGING_DIR/"
cp "$TARGET_DIR_32/$SERVICE_DLL" "$STAGING_DIR/$SERVICE_DLL_32"
cp "$TARGET_DIR_ARM64/$SERVICE_DLL" "$STAGING_DIR/$SERVICE_DLL_ARM64"
cp "$FORWARDER" "$STAGING_DIR/"
cp "$TARGET_DIR_ARM64/$SETTINGS_EXE" "$STAGING_DIR/arm64/"

# Fails unless dumpbin reads `file`'s PE machine as `machine` (`8664`, `14C`,
# `AA64`).
require_pe_machine() {
    local file="$1" machine="$2" headers
    headers="$(dumpbin_read -headers "$file")"
    grep -qE "^[[:space:]]+$machine machine" <<< "$headers" ||
        fail "$file is not built for PE machine $machine"
}
# The services and the settings exes share a file name in their target
# directories, so the staged copies' PE machine is read back: the x64 build
# staged as the 32-bit or the Arm64 one passes every other check here and
# fails only in the user's host.
for pair in "$SERVICE_DLL:8664" "$SERVICE_DLL_32:14C" "$SERVICE_DLL_ARM64:AA64" \
    "$SETTINGS_EXE:8664" "arm64/$SETTINGS_EXE:AA64"; do
    require_pe_machine "$STAGING_DIR/${pair%%:*}" "${pair##*:}"
done
# The four dictionary artefacts, from this platform's committed copy that
# `make dict` writes; an empty one would be an engine with no words.
for artifact in dictionary.fst dictionary.bin association.bin syllables.fst; do
    source_file="$DICTIONARIES_SOURCE_DIR/$artifact"
    [[ -s "$source_file" ]] || fail "missing or empty dictionary artifact $source_file (run 'make dict' + 'make build')"
    cp "$source_file" "$STAGING_DIR/Dictionaries/"
done
# Every face in the directory — which faces exist is CandidateFontChoice's
# to state (macos/scripts/bundle-app.sh:177-207). The directory is the repo-root
# `assets/fonts/font`, the one copy of the bytes every platform packages.
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
# list, vendored — the same names for x64 and Arm64, measured 2026-11-07).
# Every entry must be there — a missing DLL is a window that will not open on
# the user's machine — and every top-level DLL must be the exe's architecture:
# this read-back, not `clear_runtime_extraction`, is what catches a runtime
# staged from the other architecture's extraction.
RUNTIME_LIST="$WINDOWS_DIR/build-support/windows-app-runtime-files.txt"
stage_runtime() {
    local source_dir="$1" destination="$STAGING_DIR/$2" machine="$3" name count=0 dll
    mkdir -p "$destination"
    while IFS= read -r name; do
        name="${name%%$'\r'}"
        [[ -z "$name" || "$name" == \#* ]] && continue
        [[ -e "$source_dir/$name" ]] || fail "missing Windows App Runtime file $name in $source_dir (windows-reactor-setup staging)"
        cp -R "$source_dir/$name" "$destination/"
        count=$((count + 1))
    done < "$RUNTIME_LIST"
    [[ $count -gt 0 ]] || fail "no Windows App Runtime entries in $RUNTIME_LIST"
    for dll in "$destination"/*.dll; do
        [[ -e "$dll" ]] || fail "no top-level DLL staged in $destination"
        require_pe_machine "$dll" "$machine"
    done
}
stage_runtime "$TARGET_DIR" Runtime 8664
# The installer picks this set on an Arm64 machine (Phase 4).
stage_runtime "$TARGET_DIR_ARM64" RuntimeArm64 AA64

[[ "$skip_sign" == true ]] || echo "==> Signing the binaries"
sign_file "$STAGING_DIR/$SERVICE_DLL"
sign_file "$STAGING_DIR/$SERVICE_DLL_32"
sign_file "$STAGING_DIR/$SERVICE_DLL_ARM64"
sign_file "$STAGING_DIR/$FORWARDER_DLL"
sign_file "$STAGING_DIR/$SETTINGS_EXE"
sign_file "$STAGING_DIR/arm64/$SETTINGS_EXE"

echo "==> Compiling the installer"
STAGING_WIN="$(windows_path "$STAGING_DIR")"
OUTPUT_WIN="$(windows_path "$DISTRIBUTION_DIR")"
"$ISCC_BIN" -Q "-DAppVersion=$SHORT_VERSION" "-DDist=$STAGING_WIN" \
    "-DProductNameStringId=$PRODUCT_NAME_STRING_ID" "-O$OUTPUT_WIN" \
    "$(windows_path "$INSTALLER_SCRIPT")" ||
    fail "iscc failed"
BUILT_EXE="$DISTRIBUTION_DIR/$APP_NAME-$SHORT_VERSION.exe"
[[ -f "$BUILT_EXE" ]] || fail "iscc produced no $BUILT_EXE"
# The installer's own VERSIONINFO (the .iss VersionInfo* directives) is what
# an installed copy verifies a downloaded package by.
require_version_info "$BUILT_EXE"

[[ "$skip_sign" == true ]] || echo "==> Signing the installer"
sign_file "$BUILT_EXE"

if [[ "$BUILT_EXE" != "$OUTPUT_EXE" ]]; then
    mv -f "$BUILT_EXE" "$OUTPUT_EXE"
fi
echo ""
echo "✓ $OUTPUT_EXE"
echo "  version   $SHORT_VERSION"
echo "  commit    $HEAD_COMMIT"
echo "  installs  %ProgramFiles%\\TaigiKeyboard (administrator prompt)"
echo "  sha256    $(release_sha256 "$OUTPUT_EXE")"
declare -a PUBLISH_ARGS=(--installer "$OUTPUT_EXE")
if [[ "$skip_sign" == true ]]; then
    PUBLISH_ARGS+=(--allow-unsigned)
    echo ""
    echo "  ⚠ unsigned — the current release channel. SmartScreen typically warns (an unsigned file"
    echo "    inherits no reputation, so every version starts over) and Win11 Smart App Control can"
    echo "    refuse it outright. Installed copies DO still update in-app: with no signature to pin a"
    echo "    package to, the manifest's SHA-256 is what admits the download."
fi

if [[ "$publish" == true ]]; then
    [[ "$(git -C "$REPOSITORY_DIR" rev-parse --short HEAD)" == "$HEAD_COMMIT" &&
        -z "$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)" ]] ||
        fail "the working tree changed during the build — this installer no longer matches $HEAD_COMMIT, rebuild before publishing"
    echo ""
    bash "$WINDOWS_DIR/scripts/publish-release.sh" "${PUBLISH_ARGS[@]}"
else
    echo ""
    # %q per argument: the distribution path can hold a space, and a command
    # printed for pasting has to survive being pasted.
    echo "  Publish it with: bash windows/scripts/publish-release.sh $(printf '%q ' "${PUBLISH_ARGS[@]}")"
fi
