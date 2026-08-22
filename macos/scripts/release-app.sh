#!/usr/bin/env bash
# Produce the signed, notarized `.pkg` that the macOS input method is
# distributed as. macOS input methods cannot ship on the Mac App Store, so a
# Developer ID installer package downloaded from the web is the only route;
# docs/architecture/macos-release.md covers the one-time account setup this
# script assumes (two certificates and a stored notarytool profile).
#
# Usage: release-app.sh [--allow-dirty] [--skip-notarize] [--force]
#
#   --allow-dirty     Build from a dirty working tree. The output is named
#                     `-dirty` and must not be published.
#   --skip-notarize   Stop after signing the package. The output is named
#                     `-unnotarized`; Gatekeeper blocks it on any other Mac.
#   --force           Overwrite an existing package with the same name.
#
# Environment overrides, the first two only needed when a certificate cannot be
# resolved unambiguously:
#
#   DEVELOPER_ID_APPLICATION  certificate common name used to sign the bundle
#   DEVELOPER_ID_INSTALLER    certificate common name used to sign the package
#   NOTARY_PROFILE            notarytool keychain profile (default: TaigiKeyboard)

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"

STAGING_DIR="$DISTRIBUTION_DIR/staging"
# pkgbuild takes the *contents* of --root and lays them down under
# --install-location, so this directory must hold the `.app` and nothing else.
STAGING_ROOT="$STAGING_DIR/root"

# Written as an absolute path, but the distribution below enables only the
# current-user-home domain, which rebases it — the package installs into
# `$INSTALL_DIR`, without asking for an administrator password. This is what
# vChewing and MacishType both do; azooKey-Desktop is the outlier that installs
# system-wide.
COMPONENT_INSTALL_LOCATION="/Library/Input Methods"

NOTARY_PROFILE="${NOTARY_PROFILE:-TaigiKeyboard}"

allow_dirty=false
skip_notarize=false
force_overwrite=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --allow-dirty) allow_dirty=true ;;
        --skip-notarize) skip_notarize=true ;;
        --force) force_overwrite=true ;;
        *)
            echo "error: unknown argument '$1'" >&2
            echo "usage: release-app.sh [--allow-dirty] [--skip-notarize] [--force]" >&2
            exit 2
            ;;
    esac
    shift
done

fail() {
    echo "error: $*" >&2
    exit 1
}

# ---------------------------------------------------------------------------
# Preflight. Everything that can be known before the build is checked before
# the build, because the build plus the notarization round trip is minutes.
# ---------------------------------------------------------------------------

# The marketing version names the download; the build version is what the
# Installer compares between releases, so it is the one pkgbuild gets — and the
# only one with a format pkgbuild constrains.
[[ -n "$SHORT_VERSION" ]] || fail "CFBundleShortVersionString is empty in $SOURCE_PLIST"
[[ "$BUILD_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
    fail "CFBundleVersion '$BUILD_VERSION' is not a dotted-integer package version"

echo "==> Checking the working tree"
# Untracked files count: SwiftPM compiles everything under Sources/, so an
# uncommitted source file would otherwise ship inside a package that claims to
# be HEAD. Submodules count for the same reason.
#
# A clean tree proves the bundle ships exactly what is committed at HEAD. It
# does NOT prove the committed xcframework and dictionary artifacts were
# regenerated from the committed sources — nothing here can prove that, so run
# `make build` (and `make dict` when dictionary sources moved) beforehand.
TREE_STATUS="$(git -C "$REPOSITORY_DIR" status --porcelain --ignore-submodules=none)"
if [[ -n "$TREE_STATUS" ]]; then
    if [[ "$allow_dirty" == false ]]; then
        echo "$TREE_STATUS" >&2
        fail "working tree is dirty — commit first, or pass --allow-dirty for a throwaway build"
    fi
    echo "  ⚠ dirty tree — this package is a throwaway, do not publish it"
fi
HEAD_COMMIT="$(git -C "$REPOSITORY_DIR" rev-parse --short HEAD)"

# Anything that makes the package unpublishable is spelled out in its name, so a
# throwaway can never be mistaken for the file people are told to download.
QUALIFIER=""
if [[ -n "$TREE_STATUS" ]]; then
    QUALIFIER="$QUALIFIER-dirty"
fi
if [[ "$skip_notarize" == true ]]; then
    QUALIFIER="$QUALIFIER-unnotarized"
fi
OUTPUT_PKG="$DISTRIBUTION_DIR/$APP_NAME-$SHORT_VERSION$QUALIFIER.pkg"

# One stat, so it comes before the keychain and network checks below.
if [[ -e "$OUTPUT_PKG" && "$force_overwrite" == false ]]; then
    fail "$OUTPUT_PKG already exists — bump the version in $SOURCE_PLIST, or pass --force"
fi

echo "==> Resolving signing identities"
# `security find-identity -v` lists only valid identities, one per line, as
# `  1) <SHA-1> "<common name>"`. pkgbuild --sign matches on the common name, so
# that is what this returns — but it counts *fingerprints*, because a renewed
# Developer ID certificate is valid alongside the one it replaces under the very
# same common name. Collapsing those two would leave the choice of certificate
# to whichever one codesign happened to find first.
resolve_identity() {
    local prefix="$1" variable="$2"
    local override="${!variable:-}"
    if [[ -n "$override" ]]; then
        printf '%s' "$override"
        return
    fi

    local -a identities=()
    while IFS= read -r identity; do
        identities+=("$identity")
    done < <(security find-identity -v |
        sed -n "s/^ *[0-9][0-9]*) \([0-9A-F]*\) \"\($prefix:.*\)\"$/\1 \2/p" |
        sort -u)

    if [[ ${#identities[@]} -eq 0 ]]; then
        fail "no valid '$prefix' certificate in the keychain — see docs/architecture/macos-release.md"
    fi
    if [[ ${#identities[@]} -gt 1 ]]; then
        printf "error: %d valid '%s' certificates; set %s to the one to sign with, and\n" \
            "${#identities[@]}" "$prefix" "$variable" >&2
        printf "       remove the others from the keychain if they share a name:\n" >&2
        printf '  %s\n' "${identities[@]}" >&2
        exit 1
    fi
    # Drop the fingerprint; the trailing field is the common name.
    printf '%s' "${identities[0]#* }"
}

APPLICATION_IDENTITY="$(resolve_identity "Developer ID Application" DEVELOPER_ID_APPLICATION)"
INSTALLER_IDENTITY="$(resolve_identity "Developer ID Installer" DEVELOPER_ID_INSTALLER)"
echo "  app: $APPLICATION_IDENTITY"
echo "  pkg: $INSTALLER_IDENTITY"

if [[ "$skip_notarize" == false ]]; then
    echo "==> Checking the notarization credentials"
    # Cheapest call that proves the stored profile authenticates. Without it the
    # first notarization failure would arrive after the whole build.
    if ! xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" \
        --output-format json > /dev/null 2>&1; then
        echo "error: notarytool profile '$NOTARY_PROFILE' is missing or cannot authenticate." >&2
        echo "       Store it once with:" >&2
        echo "         xcrun notarytool store-credentials \"$NOTARY_PROFILE\" \\" >&2
        echo "           --apple-id <apple-id> --team-id <team-id> --password <app-specific-password>" >&2
        exit 1
    fi
fi

if [[ -d "$INSTALLED_APP" ]]; then
    # Not a conflict — the package installs to the same place, so it upgrades
    # whatever is there. Worth saying out loud only because a local `make
    # install` build is about to be replaced by a release one.
    echo "  note: installing this package will replace $INSTALLED_APP"
fi

# ---------------------------------------------------------------------------
# Build.
# ---------------------------------------------------------------------------

rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_ROOT"

bash "$PACKAGE_DIR/scripts/bundle-app.sh" release --sign "$APPLICATION_IDENTITY"

echo "==> Verifying the signature"
# `--deep` is wrong for signing but right for verifying: it walks whatever
# nested content exists rather than trusting that there is none.
codesign --verify --deep --strict --verbose=2 "$BUILT_APP"
SIGNATURE_DETAILS="$(codesign --display --verbose=4 "$BUILT_APP" 2>&1)"
# Hardened runtime shows up as `flags=0x10000(runtime)`. Without it the package
# notarizes into a rejection, several minutes later.
grep -q 'flags=.*runtime' <<< "$SIGNATURE_DETAILS" ||
    fail "signature has no hardened-runtime flag: $(grep 'flags=' <<< "$SIGNATURE_DETAILS")"
grep -q '^Timestamp=' <<< "$SIGNATURE_DETAILS" ||
    fail "signature carries no secure timestamp"
# What was asked for is not what was signed with: an override, or a keychain
# holding a similarly named certificate, can put an Apple Development identity
# here instead. Notarization would reject it — several minutes later.
APP_AUTHORITY="$(sed -n 's/^Authority=//p' <<< "$SIGNATURE_DETAILS" | head -1)"
[[ "$APP_AUTHORITY" == "Developer ID Application:"* ]] ||
    fail "bundle was signed by '$APP_AUTHORITY', not a Developer ID Application certificate"
APP_TEAM="$(sed -n 's/^TeamIdentifier=//p' <<< "$SIGNATURE_DETAILS" | head -1)"
[[ -n "$APP_TEAM" && "$APP_TEAM" != "not set" ]] ||
    fail "bundle signature carries no team identifier"
# The bundle claims no entitlements at all; one appearing means something was
# added without the sandbox/hardened-runtime consequences being thought through.
ENTITLEMENTS="$(codesign --display --entitlements - --xml "$BUILT_APP" 2>/dev/null || true)"
[[ -z "$ENTITLEMENTS" ]] || fail "unexpected entitlements on the bundle: $ENTITLEMENTS"

echo "==> Staging $APP_NAME.app"
# ditto rather than cp: it is the copy that preserves everything a signature
# covers, including extended attributes.
ditto "$BUILT_APP" "$STAGING_ROOT/$APP_NAME.app"
codesign --verify --strict "$STAGING_ROOT/$APP_NAME.app"

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
    <!-- arm64 only: the engine xcframework carries a single macos-arm64 slice,
         and without this productbuild would advertise x86_64 too, letting an
         Intel Mac install a bundle it cannot run. -->
    <options customize="never" hostArchitectures="arm64"/>
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
    --sign "$INSTALLER_IDENTITY" \
    --timestamp \
    "$STAGED_PKG"

echo "==> Verifying the package signature"
# Before notarization, not after: an installer certificate of the wrong type, or
# one belonging to another team, is otherwise only discovered by the notary
# service at the end of a full upload.
# Exits non-zero on an unsigned package, which the authority check below
# reports far more usefully than an unexplained abort here would.
PACKAGE_SIGNATURE="$(pkgutil --check-signature "$STAGED_PKG" || true)"
echo "$PACKAGE_SIGNATURE"
PACKAGE_AUTHORITY="$(sed -n 's/^ *1\. //p' <<< "$PACKAGE_SIGNATURE" | head -1)"
[[ "$PACKAGE_AUTHORITY" == "Developer ID Installer:"* ]] ||
    fail "package was signed by '$PACKAGE_AUTHORITY', not a Developer ID Installer certificate"
# Certificate common names end in the team identifier in parentheses, which is
# the only place the package signature exposes it.
PACKAGE_TEAM="$(sed -n 's/^.*(\([A-Z0-9]*\))$/\1/p' <<< "$PACKAGE_AUTHORITY")"
[[ "$PACKAGE_TEAM" == "$APP_TEAM" ]] ||
    fail "package team '$PACKAGE_TEAM' does not match bundle team '$APP_TEAM' — the two certificates belong to different accounts"

# ---------------------------------------------------------------------------
# Notarize. Only the package: it is the only artifact that is distributed, and
# stapling it is what lets Gatekeeper clear the install offline.
# ---------------------------------------------------------------------------

if [[ "$skip_notarize" == false ]]; then
    echo "==> Notarizing (minutes)"
    SUBMISSION_JSON="$STAGING_DIR/notarization-submit.json"
    SUBMISSION_STDERR="$STAGING_DIR/notarization-submit.err"
    # `--wait` can still exit 0 on a rejection, so the recorded status decides,
    # not the exit code.
    SUBMIT_EXIT=0
    xcrun notarytool submit "$STAGED_PKG" \
        --keychain-profile "$NOTARY_PROFILE" \
        --wait --no-progress --output-format json \
        > "$SUBMISSION_JSON" 2> "$SUBMISSION_STDERR" || SUBMIT_EXIT=$?

    # Both fields in one reader, and empty rather than raising: a submission
    # that never reached the service leaves no JSON behind, and that has to read
    # as a plain failure rather than a traceback.
    {
        read -r SUBMISSION_STATUS
        read -r SUBMISSION_ID
    } < <(python3 -c '
import json, sys
try:
    data = json.load(open(sys.argv[1]))
except Exception:
    data = {}
for key in ("status", "id"):
    print(data.get(key) or "")
' "$SUBMISSION_JSON")

    if [[ -z "$SUBMISSION_STATUS" ]]; then
        echo "error: notarytool submit failed before reporting a status (exit $SUBMIT_EXIT)" >&2
        [[ -s "$SUBMISSION_STDERR" ]] && tail -20 "$SUBMISSION_STDERR" >&2
        exit 1
    fi

    # Everything the notary service objected to, saved and summarised. Without
    # it a rejection reports only that it happened, and the next run rediscovers
    # the same objection after the same wait.
    report_notarization_issues() {
        [[ -n "$SUBMISSION_ID" ]] || return 0
        local log="$STAGING_DIR/notarization-log.json"
        xcrun notarytool log "$SUBMISSION_ID" --keychain-profile "$NOTARY_PROFILE" "$log" || return 0
        [[ -s "$log" ]] || return 0
        echo "       full log: $log" >&2
        python3 -c '
import json, sys
try:
    issues = json.load(open(sys.argv[1])).get("issues") or []
except Exception:
    issues = []
for issue in issues[:10]:
    print("       {}: {} ({})".format(
        issue.get("severity", "?"), issue.get("message", "?"), issue.get("path", "?")))
' "$log" >&2
    }

    if [[ "$SUBMISSION_STATUS" != "Accepted" ]]; then
        echo "error: notarization returned '$SUBMISSION_STATUS' (submission ${SUBMISSION_ID:-unknown})" >&2
        report_notarization_issues
        exit 1
    fi
    echo "  accepted (submission $SUBMISSION_ID)"

    echo "==> Stapling"
    # The ticket is occasionally not servable the instant notarization reports
    # Accepted; a rejection would already have exited above, so retrying here
    # can only be waiting out that gap.
    for attempt in 1 2 3; do
        if xcrun stapler staple "$STAGED_PKG"; then
            break
        fi
        [[ $attempt -eq 3 ]] && fail "could not staple the notarization ticket"
        echo "  ticket not ready, retrying in 15s"
        sleep 15
    done

    echo "==> Verifying the notarized package"
    xcrun stapler validate "$STAGED_PKG"
    spctl --assess --type install --verbose=2 "$STAGED_PKG"
else
    echo "==> Skipping notarization (--skip-notarize)"
fi

# Published only once every check above has passed, so a half-finished package
# can never be left sitting under the name people are told to download.
mv "$STAGED_PKG" "$OUTPUT_PKG"

echo ""
echo "✓ $OUTPUT_PKG"
echo "  version   $SHORT_VERSION (build $BUILD_VERSION)"
echo "  commit    $HEAD_COMMIT"
echo "  installs  $INSTALL_DIR/$APP_NAME.app (no administrator password)"
echo "  sha256    $(shasum -a 256 "$OUTPUT_PKG" | cut -d' ' -f1)"
if [[ "$skip_notarize" == true ]]; then
    echo ""
    echo "  ⚠ not notarized — Gatekeeper blocks this on every Mac but this one."
fi
