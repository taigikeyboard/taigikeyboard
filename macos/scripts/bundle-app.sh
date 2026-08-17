#!/usr/bin/env bash
# Assemble the macOS input-method `.app` from the SwiftPM executable and
# validate it. There is no Xcode project (`.pbxproj` is user-only in this repo),
# so this script is the only thing that turns build products into a bundle.
#
# Usage: bundle-app.sh [debug|release]   (default: debug)
#
# Every check here exists because the failure it catches is invisible until the
# input method is installed and silently receives no key events.

set -euo pipefail

CONFIGURATION="${1:-debug}"
if [[ "$CONFIGURATION" != "debug" && "$CONFIGURATION" != "release" ]]; then
    echo "error: configuration must be 'debug' or 'release', got '$CONFIGURATION'" >&2
    exit 2
fi

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
python3 "$(cd .. && pwd)/tools/i18n/check.py"

echo "==> Building ($CONFIGURATION)"
swift build --configuration "$CONFIGURATION" --product "$EXECUTABLE_NAME"
BUILT_EXECUTABLE="$(swift build --configuration "$CONFIGURATION" --show-bin-path)/$EXECUTABLE_NAME"
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

echo "==> Copying app icon"
# Named by Info.plist twice: CFBundleIconFile (Finder, System Settings) and
# tsInputMethodIconFileKey (the menu-bar input-source item). A missing icon
# leaves the input method with a generic placeholder in both places.
ICON_FILE="$PACKAGE_DIR/App/AppIcon.icns"
if [[ ! -s "$ICON_FILE" ]]; then
    echo "error: missing or empty app icon $ICON_FILE" >&2
    exit 1
fi
cp "$ICON_FILE" "$CONTENTS_DIR/Resources/AppIcon.icns"

echo "==> Copying dictionary data"
# Read from the iOS resource directory rather than keeping a third committed
# copy of ~24MB of generated data. `make dict` regenerates these in place, so
# both platforms bundle the same build of the dictionary by construction.
DICTIONARY_SOURCE_DIR="$(cd "$PACKAGE_DIR/.." && pwd)/ios/Resources/Dictionaries"
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

echo "==> Linting Info.plist"
plutil -lint "$CONTENTS_DIR/Info.plist"

echo "==> Checking architecture"
# The xcframework carries a single macos-arm64 slice, so an executable without
# arm64 could not have linked the Rust engine at all.
ARCHITECTURES="$(lipo -archs "$CONTENTS_DIR/MacOS/$EXECUTABLE_NAME")"
if [[ " $ARCHITECTURES " != *" arm64 "* ]]; then
    echo "error: executable has architectures '$ARCHITECTURES', expected arm64" >&2
    exit 1
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

echo "==> Signing (ad-hoc)"
# Ad-hoc is enough for a local install; Developer ID + notarization is a
# user-gated distribution step.
codesign --force --sign - --timestamp=none "$APP_DIR"
codesign --verify --strict --verbose=2 "$APP_DIR"

echo ""
echo "✓ $APP_DIR ($BUNDLE_IDENTIFIER, $ARCHITECTURES)"
