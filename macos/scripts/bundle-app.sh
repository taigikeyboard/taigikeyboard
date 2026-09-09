#!/usr/bin/env bash
# Assemble the macOS input-method `.app` from the SwiftPM executable and
# validate it. There is no Xcode project (`.pbxproj` is user-only in this repo),
# so this script is the only thing that turns build products into a bundle.
#
# Usage: bundle-app.sh [debug|release] [--sign <identity>]   (default: debug, ad-hoc)
#
# `--sign` names the code-signing identity to use. Omitted, the bundle is signed
# ad-hoc, which is all a local install needs. release-app.sh passes a Developer
# ID Application identity, which additionally opts the bundle into the hardened
# runtime and a secure timestamp — both prerequisites for notarization.
#
# Every check here exists because the failure it catches is invisible until the
# input method is installed and silently receives no key events.

set -euo pipefail

usage_error() {
    echo "error: $*" >&2
    echo "usage: bundle-app.sh [debug|release] [--sign <identity>]" >&2
    exit 2
}

# The configuration stays positional and first, so `--sign` can never be read as
# one: `bundle-app.sh --sign X` would otherwise sign a *debug* build with a
# Developer ID certificate.
CONFIGURATION="${1:-debug}"
if [[ $# -gt 0 ]]; then
    shift
fi
if [[ "$CONFIGURATION" != "debug" && "$CONFIGURATION" != "release" ]]; then
    usage_error "configuration must be 'debug' or 'release', got '$CONFIGURATION'"
fi

# `-` is codesign's own spelling for an ad-hoc signature, so the default flows
# straight through to the `codesign --sign` call below.
SIGNING_IDENTITY="-"
if [[ "${1:-}" == "--sign" ]]; then
    [[ $# -ge 2 ]] || usage_error "--sign needs an identity"
    SIGNING_IDENTITY="$2"
    shift 2
fi
[[ $# -eq 0 ]] || usage_error "unexpected argument '$1'"

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"

APP_DIR="$BUILT_APP"
CONTENTS_DIR="$APP_DIR/Contents"

cd "$PACKAGE_DIR"

echo "==> Checking generated i18n strings"
# The Swift under Sources/TaigiInputMethodCore/Strings/Generated comes from i18n/*.json via the
# repo-root `make i18n`. Checked here rather than only as a Makefile prerequisite because this script
# runs its own `swift build` below: a bundle assembled by calling the script directly would otherwise
# ship strings that no longer match their source. Same role as Android's Gradle checkI18nGenerated.
python3 "$REPOSITORY_DIR/tools/i18n/check.py"

echo "==> Building ($CONFIGURATION)"
# One universal `.pkg` serves both Mac architectures, so a release bundle has to
# carry both. Debug is the local dev loop and stays native: compiling the whole
# dependency graph a second time for an architecture this Mac cannot execute
# costs every iteration and buys nothing. release-app.sh always passes
# `release`, so the release path cannot fall into the native branch.
if [[ "$CONFIGURATION" == "release" ]]; then
    EXPECTED_ARCHITECTURES="arm64,x86_64"
    # One native build per architecture, then `lipo` — not the Swift Build
    # backend's multi-architecture mode, which cannot link this package at all.
    # Why, and what it fails with: docs/architecture/macos-release.md
    # § Architectures.
    SLICE_EXECUTABLES=()
    for slice_arch in ${EXPECTED_ARCHITECTURES//,/ }; do
        # Same arguments as the build below: the product directory is
        # per-architecture, so a differently-spelled query would answer about a
        # different build.
        slice_executable="$(swift build --configuration "$CONFIGURATION" \
            --arch "$slice_arch" --show-bin-path)/$EXECUTABLE_NAME"
        # Deleting only the final executable forces this run to relink it, while
        # every object and module cache stays. A successful `swift build` means
        # SwiftPM considered the graph up to date — which is not the same as
        # "this executable was produced from what is checked out now", and the
        # architecture and symbol assertions below cannot tell a complete
        # universal binary built from last week's sources from today's.
        rm -f "$slice_executable"
        swift build --configuration "$CONFIGURATION" --arch "$slice_arch" \
            --product "$EXECUTABLE_NAME"
        if [[ ! -x "$slice_executable" ]]; then
            echo "error: $slice_arch build left no executable at $slice_executable" >&2
            exit 1
        fi
        SLICE_EXECUTABLES+=("$slice_executable")
    done
    BUILT_EXECUTABLE="$PACKAGE_DIR/.build/universal-$CONFIGURATION/$EXECUTABLE_NAME"
    mkdir -p "$(dirname "$BUILT_EXECUTABLE")"
    lipo -create "${SLICE_EXECUTABLES[@]}" -output "$BUILT_EXECUTABLE"
else
    EXPECTED_ARCHITECTURES="$(uname -m)"
    swift build --configuration "$CONFIGURATION" --product "$EXECUTABLE_NAME"
    BUILT_EXECUTABLE="$(swift build --configuration "$CONFIGURATION" --show-bin-path)/$EXECUTABLE_NAME"
fi
if [[ ! -x "$BUILT_EXECUTABLE" ]]; then
    echo "error: built executable not found at $BUILT_EXECUTABLE" >&2
    exit 1
fi

echo "==> Assembling $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$CONTENTS_DIR/MacOS" "$CONTENTS_DIR/Resources"
cp "$BUILT_EXECUTABLE" "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME"
cp "$SOURCE_PLIST" "$CONTENTS_DIR/Info.plist"
printf 'APPL????' > "$CONTENTS_DIR/PkgInfo"

echo "==> Copying icons"
# The two files Info.plist names, one per icon key; Info.plist itself explains
# why there are two. Either one missing leaves a generic placeholder in its own
# place, which shows up only once the input method is installed.
# Regenerate AppIcon.icns with `swift tools/desktop/make-app-icon.swift`.
ICON_FILE="$PACKAGE_DIR/App/AppIcon.icns"
if [[ ! -s "$ICON_FILE" ]]; then
    echo "error: missing or empty app icon $ICON_FILE" >&2
    exit 1
fi
cp "$ICON_FILE" "$CONTENTS_DIR/Resources/AppIcon.icns"

# Named rather than hardcoded: renaming the icon in Info.plist would otherwise
# ship a bundle that passes every check here and still shows a placeholder.
MENU_BAR_ICON_FILE="$PACKAGE_DIR/App/$MENU_BAR_ICON_NAME"
if [[ ! -s "$MENU_BAR_ICON_FILE" ]]; then
    echo "error: missing or empty menu-bar icon $MENU_BAR_ICON_FILE (regenerate with 'swift scripts/make-menubar-icon.swift')" >&2
    exit 1
fi
cp "$MENU_BAR_ICON_FILE" "$CONTENTS_DIR/Resources/$MENU_BAR_ICON_NAME"

echo "==> Compiling localized bundle names"
# What macOS matches the SYSTEM language against to name this input source;
# without them it falls back to the untranslated `CFBundleName`. Why, and which
# languages: MACOS_BUNDLE_LOCALIZATIONS in tools/i18n/i18n_lib.py.
#
# Converted rather than copied: a binary plist is the form Apple's own apps ship
# their compiled `.strings` in, and the conversion is what turns a malformed file
# into a failed build instead of an input source that silently loses its name.
#
# Generated by `make i18n`, so this loop must not name the languages — which ones
# exist is that generator's to state.
BUNDLE_NAME_DIRECTORIES=("$PACKAGE_DIR"/App/*.lproj)
if [[ ! -d "${BUNDLE_NAME_DIRECTORIES[0]}" ]]; then
    echo "error: no App/*.lproj in $PACKAGE_DIR (run 'make i18n')" >&2
    exit 1
fi
for localization_dir in "${BUNDLE_NAME_DIRECTORIES[@]}"; do
    source_strings="$localization_dir/InfoPlist.strings"
    if [[ ! -s "$source_strings" ]]; then
        echo "error: missing or empty $source_strings (run 'make i18n')" >&2
        exit 1
    fi
    destination_dir="$CONTENTS_DIR/Resources/$(basename "$localization_dir")"
    mkdir -p "$destination_dir"
    plutil -convert binary1 "$source_strings" -o "$destination_dir/InfoPlist.strings"
done

echo "==> Copying dictionary data"
# The repo-root `dictionaries`, the one copy of the bytes every platform
# packages — nothing to sync, nothing to drift.
DICTIONARY_SOURCE_DIR="$REPOSITORY_DIR/dictionaries"
for artifact in dictionary.fst dictionary.bin association.bin syllables.fst; do
    source_file="$DICTIONARY_SOURCE_DIR/$artifact"
    # Fail here rather than ship a bundle whose input method launches, receives
    # keys, and produces no candidates at all.
    if [[ ! -s "$source_file" ]]; then
        echo "error: missing or empty dictionary artifact $source_file (run 'make dict')" >&2
        exit 1
    fi
    cp "$source_file" "$CONTENTS_DIR/Resources/$artifact"
done

echo "==> Copying symbol table"
# The symbol picker's table, from the repo-root `symbols` — the one copy both
# desktop platforms read (Windows compiles it in). Fail here rather than ship
# a chord that opens nothing: `SymbolTable.bundled` logs and stays nil.
SYMBOL_TABLE_FILE="$REPOSITORY_DIR/symbols/desktop-symbols.json"
if [[ ! -s "$SYMBOL_TABLE_FILE" ]]; then
    echo "error: missing or empty symbol table $SYMBOL_TABLE_FILE" >&2
    exit 1
fi
cp "$SYMBOL_TABLE_FILE" "$CONTENTS_DIR/Resources/desktop-symbols.json"

echo "==> Copying fonts"
# The typefaces the candidate-window font picker offers, laid out under the
# directory Info.plist's ATSApplicationFontsPath names, which is what AppKit
# activates at launch. They come from the repo-root `fonts/font`, the one copy
# of the bytes every platform packages — nothing to sync, nothing to drift.
#
# The whole directory, deliberately without a list of filenames: which faces
# exist is `CandidateFontChoice`'s to state, not this script's, and a second
# roster here is one a new case could be added to only one of.
FONT_SOURCE_DIR="$REPOSITORY_DIR/fonts/font"
FONT_DESTINATION_DIR="$CONTENTS_DIR/Resources/$APPLICATION_FONTS_PATH"
FONT_FILES=()
for candidate in "$FONT_SOURCE_DIR"/*.ttf "$FONT_SOURCE_DIR"/*.otf; do
    # Unmatched globs stay literal under this shell's options, so a missing
    # directory has to be caught by testing the paths themselves — which also
    # rejects a truncated file rather than shipping one nothing can activate.
    [[ -s "$candidate" ]] && FONT_FILES+=("$candidate")
done
if [[ ${#FONT_FILES[@]} -eq 0 ]]; then
    echo "error: no font files in $FONT_SOURCE_DIR — the font picker would draw every option in the system font" >&2
    exit 1
fi
mkdir -p "$FONT_DESTINATION_DIR"
cp "${FONT_FILES[@]}" "$FONT_DESTINATION_DIR/"

echo "==> Linting Info.plist"
plutil -lint "$CONTENTS_DIR/Info.plist"

echo "==> Checking architecture"
# The exact set, sorted — not "contains arm64". A release bundle that lost
# x86_64 still contains arm64, and the only machine that would ever notice is an
# Intel Mac, at install time, in a user's hands.
ARCHITECTURES="$(lipo -archs "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME" |
    tr ' ' '\n' | LC_ALL=C sort | paste -sd, -)"
if [[ "$ARCHITECTURES" != "$EXPECTED_ARCHITECTURES" ]]; then
    echo "error: executable has architectures '$ARCHITECTURES', expected '$EXPECTED_ARCHITECTURES'" >&2
    exit 1
fi

if [[ "$CONFIGURATION" == "release" ]]; then
    echo "==> Checking deployment target per slice"
    # Info.plist claims LSMinimumSystemVersion for the whole app, but each slice
    # carries its own LC_BUILD_VERSION, and they are produced by different
    # toolchain defaults. A slice claiming an older minimum would launch on a
    # macOS the app was never built against.
    for slice_arch in ${ARCHITECTURES//,/ }; do
        # `-show-build` is the spelling vtool documents; `minos` is the
        # LC_BUILD_VERSION field. A slice carrying only the older
        # LC_VERSION_MIN_MACOSX would leave this empty, which fails closed —
        # reported as its own case so the message says so.
        SLICE_MINIMUM="$(vtool -arch "$slice_arch" -show-build \
            "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME" | sed -n 's/^ *minos *//p')"
        if [[ -z "$SLICE_MINIMUM" ]]; then
            echo "error: $slice_arch slice carries no LC_BUILD_VERSION minos to check" >&2
            exit 1
        fi
        if [[ "$SLICE_MINIMUM" != "$MINIMUM_SYSTEM_VERSION" ]]; then
            echo "error: $slice_arch slice targets macOS $SLICE_MINIMUM, but the app declares $MINIMUM_SYSTEM_VERSION" >&2
            exit 1
        fi
    done
fi

# Read once into a variable: piping into `grep -q` closes the pipe early, which
# under `pipefail` reports the whole pipeline as failed via nm's SIGPIPE.
EXPORTED_SYMBOLS="$(nm -gU "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME")"

echo "==> Checking Objective-C class names Info.plist looks up"
# InputMethodKit resolves these strings with NSClassFromString at launch. A
# rename or a dead-stripped class compiles fine and fails only on the device,
# as an input method that appears installed but never receives events.
for objc_class in "$CONTROLLER_CLASS" "$PRINCIPAL_CLASS"; do
    if ! grep -q "_OBJC_CLASS_\$_${objc_class}\$" <<< "$EXPORTED_SYMBOLS"; then
        echo "error: class '$objc_class' named in Info.plist is not in the executable" >&2
        exit 1
    fi
done

echo "==> Checking the Rust engine linked in"
if ! grep -q '__swift_bridge__\$process_request_bytes$' <<< "$EXPORTED_SYMBOLS"; then
    echo "error: swift-bridge FFI entry point missing — RustTaigi.xcframework did not link" >&2
    exit 1
fi

echo "==> Checking dynamic library dependencies"
# The bundle embeds no frameworks, so any @rpath dependency would fail to load
# once the app is copied out of the build directory.
LINKED_LIBRARIES="$(otool -L "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME")"
if grep -q '@rpath/' <<< "$LINKED_LIBRARIES"; then
    echo "error: executable has @rpath dependencies but the bundle embeds no frameworks:" >&2
    grep '@rpath/' <<< "$LINKED_LIBRARIES" >&2
    exit 1
fi

if [[ "$SIGNING_IDENTITY" == "-" ]]; then
    echo "==> Signing (ad-hoc)"
    # Enough for a local install, and deliberately without a timestamp: a
    # secure timestamp needs the network, which a dev loop should not.
    codesign --force --sign "$SIGNING_IDENTITY" --timestamp=none "$APP_DIR"
else
    echo "==> Signing ($SIGNING_IDENTITY)"
    # Hardened runtime and a secure timestamp are both required for
    # notarization. No `--deep`: it papers over nested-code signing mistakes,
    # and there is no nested code to sign — the assembly above copies one
    # executable and data files, nothing else.
    codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$APP_DIR"
fi
codesign --verify --strict --verbose=2 "$APP_DIR"

echo ""
echo "✓ $APP_DIR ($BUNDLE_IDENTIFIER, $ARCHITECTURES)"
