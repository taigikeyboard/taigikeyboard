#!/usr/bin/env bash
# The macOS installer package around an assembled input-method bundle, shared
# by release-app.sh (Developer ID signed, then notarized) and
# package-nightly.sh (unsigned nightly build). Sourced after
# bundle-identity.sh, whose identity values and `fail` it uses.

# Written as an absolute path, but the distribution below enables only the
# current-user-home domain, which rebases it — the package installs into
# `$INSTALL_DIR`, without asking for an administrator password. This is what
# vChewing and MacishType both do; azooKey-Desktop is the outlier that installs
# system-wide.
COMPONENT_INSTALL_LOCATION="/Library/Input Methods"

# Stop unless <bundle> carries both slices. A debug bundle carries one; only a
# release bundle carries both (bundle-app.sh asserts the set, this refuses the
# wrong configuration) — a package missing a slice would install and never load
# on half the Macs.
require_universal_bundle() {
    local architectures
    architectures="$(lipo -archs "$1/Contents/MacOS/$EXECUTABLE_NAME" | tr ' ' '\n' | LC_ALL=C sort | paste -sd, -)"
    [[ "$architectures" == "arm64,x86_64" ]] ||
        fail "$1 has architectures '$architectures', not arm64,x86_64 — rebuild it with 'bundle-app.sh release'"
}

# Build `<staging dir>/$APP_NAME.pkg` from the bundle in `<staging dir>/root`
# and set STAGED_PKG to it. With an installer identity the product archive is
# signed and timestamped with it; without one it is unsigned. Checking the
# signature is the caller's job.
#
#   build_installer_package <staging dir> [installer identity]
build_installer_package() {
    local STAGING_DIR="$1" INSTALLER_IDENTITY="${2:-}"
    local STAGING_ROOT="$STAGING_DIR/root"
    local COMPONENT_PLIST SCRIPTS_DIR COMPONENT_PKG DISTRIBUTION_XML

    echo "==> Describing the package component"
    COMPONENT_PLIST="$STAGING_DIR/component.plist"
    # Generated rather than committed: pkgbuild itself derives the bundle path, so
    # the description cannot drift from the bundle it describes.
    pkgbuild --analyze --root "$STAGING_ROOT" "$COMPONENT_PLIST" > /dev/null
    /usr/libexec/PlistBuddy -c "Print :1" "$COMPONENT_PLIST" > /dev/null 2>&1 &&
        fail "pkgbuild found more than one bundle under $STAGING_ROOT"

    set_component_property() {
        local key="$1" type="$2" value="$3"
        /usr/libexec/PlistBuddy -c "Set :0:$key $value" "$COMPONENT_PLIST" 2>/dev/null ||
            /usr/libexec/PlistBuddy -c "Add :0:$key $type $value" "$COMPONENT_PLIST"
    }
    # Relocation is the load-bearing one: left on, the Installer would follow an
    # existing copy of this bundle identifier and write outside the input-method
    # directory entirely.
    set_component_property BundleIsRelocatable       bool   false
    set_component_property BundleHasStrictIdentifier bool   true
    set_component_property BundleIsVersionChecked    bool   true
    set_component_property BundleOverwriteAction     string upgrade

    plutil -lint "$COMPONENT_PLIST" > /dev/null
    [[ "$(/usr/libexec/PlistBuddy -c "Print :0:BundleIsRelocatable" "$COMPONENT_PLIST")" == "false" ]] ||
        fail "component plist still marks the bundle relocatable"

    echo "==> Writing the postinstall script"
    SCRIPTS_DIR="$STAGING_DIR/scripts"
    mkdir -p "$SCRIPTS_DIR"
    # The installer replaces the bundle under a running input method, which then
    # keeps serving keystrokes from the version that was just overwritten. Killing
    # it makes the next keystroke relaunch the copy that was actually installed;
    # the process is restarted on demand, so there is nothing to start here.
    #
    # A home-domain install runs its scripts as the installing user, so this reaches
    # only that user's process — which is the whole scope of a per-user install.
    cat > "$SCRIPTS_DIR/postinstall" <<POSTINSTALL
#!/bin/sh
killall $EXECUTABLE_NAME 2>/dev/null || true
exit 0
POSTINSTALL
    chmod +x "$SCRIPTS_DIR/postinstall"

    echo "==> Building the component package"
    COMPONENT_PKG="$STAGING_DIR/component.pkg"
    pkgbuild \
        --root "$STAGING_ROOT" \
        --component-plist "$COMPONENT_PLIST" \
        --scripts "$SCRIPTS_DIR" \
        --identifier "$BUNDLE_IDENTIFIER" \
        --version "$BUILD_VERSION" \
        --install-location "$COMPONENT_INSTALL_LOCATION" \
        "$COMPONENT_PKG"

    echo "==> Building the product archive"
    STAGED_PKG="$STAGING_DIR/$APP_NAME.pkg"
    DISTRIBUTION_XML="$STAGING_DIR/distribution.xml"
    # The only reason this pipeline needs productbuild: `<domains>` lives in a
    # distribution, and it is what decides both where the component lands and what
    # authorization the install needs. Enabling only the current-user-home domain
    # rebases the absolute install location under the installing user's home, where
    # no administrator password is required. No installer choices beyond that —
    # one component, `customize="never"`.
    #
    # `auth="none"` is deprecated and the domain above is what actually governs
    # authorization; it is kept because vChewing — the closest peer, also signed and
    # notarized into the home domain — still sets it, and an inert attribute is
    # cheaper than discovering some macOS version still reads it.
    #
    # Generated rather than committed for the same reason as the component plist:
    # every value in it comes from Info.plist, so it cannot drift from the bundle.
    # Nothing here escapes those values; `xmllint` below is what catches it if one
    # ever arrives carrying `&` or `<`.
    cat > "$DISTRIBUTION_XML" <<DISTRIBUTION
<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>$APP_NAME</title>
    <!-- Both architectures, listed rather than left to the default: the bundle
         is universal, and naming what it supports makes the installer's claim
         reviewable against bundle-app.sh's architecture assertion. Omitting the
         attribute lets the Installer match permissively, which would also
         accept a bundle that had silently lost a slice. -->
    <options customize="never" hostArchitectures="arm64,x86_64"/>
    <domains enable_currentUserHome="true" enable_localSystem="false" enable_anywhere="false"/>
    <volume-check>
        <allowed-os-versions>
            <os-version min="$MINIMUM_SYSTEM_VERSION"/>
        </allowed-os-versions>
    </volume-check>
    <choices-outline>
        <line choice="install"/>
    </choices-outline>
    <choice id="install" visible="false">
        <pkg-ref id="$BUNDLE_IDENTIFIER"/>
    </choice>
    <pkg-ref id="$BUNDLE_IDENTIFIER" version="$BUILD_VERSION" auth="none">component.pkg</pkg-ref>
</installer-gui-script>
DISTRIBUTION
    xmllint --noout "$DISTRIBUTION_XML"

    productbuild \
        --distribution "$DISTRIBUTION_XML" \
        --package-path "$STAGING_DIR" \
        ${INSTALLER_IDENTITY:+--sign "$INSTALLER_IDENTITY" --timestamp} \
        "$STAGED_PKG"
}
