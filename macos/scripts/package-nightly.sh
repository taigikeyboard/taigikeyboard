#!/usr/bin/env bash
# Wrap the ad-hoc signed bundle `bundle-app.sh release` assembled (`$BUILT_APP`)
# in an UNSIGNED installer package for the nightly channel
# (.github/workflows/desktop-nightly.yml, docs/architecture/nightly-builds.md).
#
# Usage: package-nightly.sh <output .pkg>
#
# The same package layout as a release (lib/installer-package.sh) with no
# Developer ID signature and no notarization, so Gatekeeper asks the tester to
# allow it once ("Open Anyway"). Holds no credential and needs none: it runs on
# the nightly workflow's credential-free macOS job.

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"
# shellcheck source=lib/installer-package.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/installer-package.sh"

[[ $# -eq 1 ]] || {
    echo "usage: package-nightly.sh <output .pkg>" >&2
    exit 2
}
OUTPUT_PKG="$1"

[[ -d "$BUILT_APP" ]] ||
    fail "no bundle at $BUILT_APP — run 'bash macos/scripts/bundle-app.sh release' first"
require_universal_bundle "$BUILT_APP"
codesign --verify --strict "$BUILT_APP"

STAGING_DIR="$DISTRIBUTION_DIR/nightly-staging"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/root"
# ditto keeps the ad-hoc signature intact (see release-app.sh).
ditto "$BUILT_APP" "$STAGING_DIR/root/$APP_NAME.app"

build_installer_package "$STAGING_DIR"

# Unsigned on purpose: a signature appearing here means a credential reached a
# job that must never hold one.
if pkgutil --check-signature "$STAGED_PKG" > /dev/null 2>&1; then
    fail "$STAGED_PKG carries a signature — the nightly package must be unsigned"
fi

mkdir -p "$(dirname "$OUTPUT_PKG")"
mv "$STAGED_PKG" "$OUTPUT_PKG"
echo "✓ $OUTPUT_PKG (unsigned, version $SHORT_VERSION build $BUILD_VERSION)"
