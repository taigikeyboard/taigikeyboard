#!/usr/bin/env bash
# Shared bundle identity, sourced by the macOS scripts.
#
# App/Info.plist is the single source of truth for who this app is and what its
# executable is called; this file is the single place that reads it and the
# single place that names the assembled bundle's path. Both the bundle script
# and the install script source it, so neither re-derives either fact.

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
        "$plist")"

    # Values are single-line and space-free by construction (bundle IDs, class
    # names); a `read` per line keeps the order explicit.
    {
        read -r APP_NAME
        read -r BUNDLE_IDENTIFIER
        read -r EXECUTABLE_NAME
        read -r CONTROLLER_CLASS
        read -r PRINCIPAL_CLASS
    } <<< "$values"
}

PACKAGE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOURCE_PLIST="$PACKAGE_DIR/App/Info.plist"

_read_bundle_identity "$SOURCE_PLIST"

# The assembled bundle, and where an install reads it from and writes it to.
BUILT_APP="$PACKAGE_DIR/.build/bundle/$APP_NAME.app"
INSTALL_DIR="$HOME/Library/Input Methods"
INSTALLED_APP="$INSTALL_DIR/$APP_NAME.app"
INSTALLED_EXECUTABLE="$INSTALLED_APP/Contents/MacOS/$EXECUTABLE_NAME"
