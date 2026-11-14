#!/usr/bin/env bash
# Produce the signed, notarized `.pkg` that the macOS input method is
# distributed as. macOS input methods cannot ship on the Mac App Store, so a
# Developer ID installer package downloaded from the web is the only route;
# docs/architecture/macos-release.md covers the one-time account setup this
# script assumes (two certificates and a notarization credential). The release
# runs it on a hosted Mac (.github/workflows/macos-build.yml).
#
# It builds nothing: it signs the bundle `bundle-app.sh release` assembled
# (`$BUILT_APP`) with Developer ID, then packages and notarizes it. The build
# runs third-party code (crate build scripts, SwiftPM packages), so on the
# release runner it happens in a job that never sees a credential, and only
# this script — Apple's tools and this repository's code — runs where the
# certificates are. Locally: `make macos-libs`, `bundle-app.sh release`, then
# this.
#
# Usage: release-app.sh [--allow-dirty] [--skip-notarize] [--force]
#
#   --allow-dirty     Package from a dirty working tree. The output is named
#                     `-dirty` and must not be published.
#   --skip-notarize   Stop after signing the package. The output is named
#                     `-unnotarized`; Gatekeeper blocks it on any other Mac.
#   --force           Overwrite an existing package with the same name.
#
# Staging the package on the draft is `publish-release.sh`, a separate step of
# the release workflow — it runs after the signing credentials are removed.
#
# Environment overrides, the first two only needed when a certificate cannot be
# resolved unambiguously:
#
#   DEVELOPER_ID_APPLICATION  certificate common name used to sign the bundle
#   DEVELOPER_ID_INSTALLER    certificate common name used to sign the package
#   NOTARY_PROFILE            notarytool keychain profile (default: TaigiKeyboard)
#
# Notarization authenticates with an App Store Connect API key when
# NOTARY_API_KEY is set — the release runner's way (.github/workflows/
# macos-build.yml) — and with the keychain profile otherwise:
#
#   NOTARY_API_KEY            path to the .p8 private key
#   NOTARY_API_KEY_ID         its key ID
#   NOTARY_API_ISSUER         the team's issuer ID

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"
# shellcheck source=lib/installer-package.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/installer-package.sh"
# shellcheck source=../../tools/release/lib/desktop-release.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../tools/release/lib/desktop-release.sh"

STAGING_DIR="$DISTRIBUTION_DIR/staging"
# pkgbuild takes the *contents* of --root and lays them down under
# --install-location, so this directory must hold the `.app` and nothing else.
STAGING_ROOT="$STAGING_DIR/root"

NOTARY_PROFILE="${NOTARY_PROFILE:-TaigiKeyboard}"
# Every notarytool call authenticates the same way.
if [[ -n "${NOTARY_API_KEY:-}" ]]; then
    [[ -f "$NOTARY_API_KEY" ]] || fail "NOTARY_API_KEY '$NOTARY_API_KEY' is not a file"
    [[ -n "${NOTARY_API_KEY_ID:-}" && -n "${NOTARY_API_ISSUER:-}" ]] ||
        fail "NOTARY_API_KEY needs NOTARY_API_KEY_ID and NOTARY_API_ISSUER"
    NOTARY_AUTH=(--key "$NOTARY_API_KEY" --key-id "$NOTARY_API_KEY_ID" --issuer "$NOTARY_API_ISSUER")
    NOTARY_AUTH_NAME="App Store Connect API key $NOTARY_API_KEY_ID"
else
    NOTARY_AUTH=(--keychain-profile "$NOTARY_PROFILE")
    NOTARY_AUTH_NAME="notarytool profile '$NOTARY_PROFILE'"
fi

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

# ---------------------------------------------------------------------------
# Preflight. Everything that can be known before signing is checked first,
# because the notarization round trip is minutes.
# ---------------------------------------------------------------------------

# The marketing version names the download; the build version is what the
# Installer compares between releases, so it is the one pkgbuild gets — and the
# only one with a format pkgbuild constrains.
[[ -n "$SHORT_VERSION" ]] || fail "CFBundleShortVersionString is empty in $SOURCE_PLIST"
[[ "$BUILD_VERSION" =~ ^[0-9]+(\.[0-9]+)*$ ]] ||
    fail "CFBundleVersion '$BUILD_VERSION' is not a dotted-integer package version"

# The plist documents the rule (MAJOR*10000 + MINOR*100 + PATCH). This enforces
# it at package time; `tools/release_notes.py check-versions` enforces the same
# rule during release prep, before any build exists. Both check because the
# failure mode of a stale build version is an Installer that silently refuses to
# upgrade, and either entry point can be the first one a release goes through.
if [[ "$SHORT_VERSION" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    # The derivation is only collision-free while minor and patch each fit their
    # own decimal field: 3.1.100 and 3.2.0 both derive to 30200, and two releases
    # sharing a build version is the same silent-refusal failure.
    for component in "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]}"; do
        ((10#$component <= 99)) ||
            fail "$SHORT_VERSION: minor and patch must be 0-99; the build version MAJOR*10000 + MINOR*100 + PATCH would collide with another release"
    done
    expected_build_version=$((10#${BASH_REMATCH[1]} * 10000 + 10#${BASH_REMATCH[2]} * 100 + 10#${BASH_REMATCH[3]}))
    [[ "$BUILD_VERSION" == "$expected_build_version" ]] ||
        fail "CFBundleVersion '$BUILD_VERSION' does not derive from $SHORT_VERSION (expected $expected_build_version)"
fi

echo "==> Checking the working tree"
# A clean tree proves the packaging — this script, the plist it reads — is what
# HEAD commits. It cannot prove the bundle was built from HEAD: this script
# builds nothing. On the release runner that proof is the `build` job's, which
# builds a fresh checkout of the staged commit and checks the build changed
# nothing tracked. A local bundle built from a dirty tree and packaged after a
# commit is a throwaway either way: only the release workflow stages.
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
OUTPUT_PKG="$DISTRIBUTION_DIR/${MACOS_ASSET%.pkg}$QUALIFIER.pkg"

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
    # Cheapest call that proves the credential authenticates. Without it the
    # first notarization failure would arrive after signing and packaging.
    if ! xcrun notarytool history "${NOTARY_AUTH[@]}" \
        --output-format json > /dev/null 2>&1; then
        echo "error: $NOTARY_AUTH_NAME is missing or cannot authenticate." >&2
        [[ -z "${NOTARY_API_KEY:-}" ]] || exit 1
        echo "       Store the profile once with:" >&2
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
# Sign the release bundle.
# ---------------------------------------------------------------------------

[[ -d "$BUILT_APP" ]] ||
    fail "no bundle at $BUILT_APP — run 'bash macos/scripts/bundle-app.sh release' first"
require_universal_bundle "$BUILT_APP"

rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_ROOT"

echo "==> Signing with $APPLICATION_IDENTITY"
# Replaces bundle-app.sh's ad-hoc signature. Hardened runtime and a secure
# timestamp are both required for notarization. No `--deep`: it papers over
# nested-code signing mistakes, and there is no nested code to sign —
# bundle-app.sh copies one executable and data files, nothing else.
codesign --force --sign "$APPLICATION_IDENTITY" --options runtime --timestamp "$BUILT_APP"

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

build_installer_package "$STAGING_DIR" "$INSTALLER_IDENTITY"

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
        "${NOTARY_AUTH[@]}" \
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
        xcrun notarytool log "$SUBMISSION_ID" "${NOTARY_AUTH[@]}" "$log" || return 0
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
echo "  sha256    $(release_sha256 "$OUTPUT_PKG")"
if [[ "$skip_notarize" == true ]]; then
    echo ""
    echo "  ⚠ not notarized — Gatekeeper blocks this on every Mac but this one."
fi

# Staging is a separate script (`publish-release.sh`): everything above is
# minutes of build and notarization that must not be repeated because a network
# step failed, and the release workflow runs it only after the signing
# credentials are gone. Its preflight re-checks the tree is clean at this commit.
