#!/usr/bin/env bash
# Shared bundle identity and the error exit that goes with it, sourced by the
# macOS scripts.
#
# App/Info.plist is the single source of truth for who this app is and what its
# executable is called; this file is the single place that reads it and the
# single place that names the assembled bundle's path. Both the bundle script
# and the install script source it, so neither re-derives either fact.

# One line on stderr and stop. Lives here rather than in each script because
# every script that sources this one needs it and an error path that drifts
# between copies is an error path nobody reads twice.
fail() {
    echo "error: $*" >&2
    exit 1
}

# One PlistBuddy invocation for every key — the same file is being read either
# way, and `make` evaluates its variables on every invocation.
_read_bundle_identity() {
    local plist="$1"
    local values
    values="$(/usr/libexec/PlistBuddy \
        -c "Print :CFBundleName" \
        -c "Print :CFBundleIdentifier" \
        -c "Print :CFBundleExecutable" \
        -c "Print :InputMethodServerControllerClass" \
        -c "Print :NSPrincipalClass" \
        -c "Print :CFBundleShortVersionString" \
        -c "Print :CFBundleVersion" \
        -c "Print :LSMinimumSystemVersion" \
        -c "Print :tsInputMethodIconFileKey" \
        -c "Print :ATSApplicationFontsPath" \
        "$plist")"

    # Values are single-line and space-free by construction (bundle IDs, class
    # names, dotted versions); a `read` per line keeps the order explicit.
    {
        read -r APP_NAME
        read -r BUNDLE_IDENTIFIER
        read -r EXECUTABLE_NAME
        read -r CONTROLLER_CLASS
        read -r PRINCIPAL_CLASS
        # The marketing version users see, then the version the macOS Installer
        # compares between releases to decide upgrade from downgrade.
        read -r SHORT_VERSION
        read -r BUILD_VERSION
        read -r MINIMUM_SYSTEM_VERSION
        # A complete filename, unlike CFBundleIconFile, so the bundle script can
        # copy it without knowing what it is called.
        read -r MENU_BAR_ICON_NAME
        # Where AppKit activates the candidate-window typefaces from, relative to
        # Contents/Resources. Read rather than hardcoded so the bundle script
        # fills the directory the plist actually names — and so deleting the key
        # fails the build here rather than shipping fonts nothing activates.
        read -r APPLICATION_FONTS_PATH
    } <<< "$values"
}

PACKAGE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPOSITORY_DIR="$(cd "$PACKAGE_DIR/.." && pwd)"
SOURCE_PLIST="$PACKAGE_DIR/App/Info.plist"

_read_bundle_identity "$SOURCE_PLIST"

# The assembled bundle, and where an install reads it from and writes it to.
# `.build/distribution` deliberately avoids `.build/release`, which SwiftPM owns
# as a symlink to its release build products.
BUILT_APP="$PACKAGE_DIR/.build/bundle/$APP_NAME.app"
DISTRIBUTION_DIR="$PACKAGE_DIR/.build/distribution"
# Where the input method ends up, for a local install and a package install
# alike: the distributed package installs into the user's home domain too.
INSTALL_DIR="$HOME/Library/Input Methods"
INSTALLED_APP="$INSTALL_DIR/$APP_NAME.app"
INSTALLED_EXECUTABLE="$INSTALLED_APP/Contents/MacOS/$EXECUTABLE_NAME"
