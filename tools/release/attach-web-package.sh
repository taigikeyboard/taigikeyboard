#!/usr/bin/env bash
# Attach one Chrome Web Store package — the ChromeOS input method or the browser
# extension — to the `desktop-<version>` draft, with its `.sha256` receipt.
#
# Usage: attach-web-package.sh <chromeos|extension> <source_sha>
#
# Run by the `attach` job of `chromeos-build.yml` / `extension-build.yml`, from
# the repository root, after the build job's artifact was downloaded to
# web/target/. The zip is `make -C web <package>`'s
# `web/target/taigi-keyboard-<package>.zip`; the asset name is
# tools/release/lib/desktop-release.sh's (CHROMEOS_ASSET / EXTENSION_ASSET).
#
# Environment: GH_TOKEN (contents: write).

set -euo pipefail

fail() { echo "error: $*" >&2; exit 1; }

[[ $# -eq 2 ]] || fail "usage: attach-web-package.sh <chromeos|extension> <source_sha>"
package="$1"
case "$package" in
    chromeos | extension) ;;
    *) fail "unknown package '$package'" ;;
esac

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# `make version-desktop` writes the manifest with the rest of the train, so its
# version names the draft.
SHORT_VERSION="$(node --print "require('$REPOSITORY_DIR/web/$package/manifest.json').version")"
DESKTOP_SOURCE_COMMIT="$2"
RELEASE_TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$RELEASE_TEMP_DIR"' EXIT
# shellcheck source=lib/desktop-release.sh
source "$REPOSITORY_DIR/tools/release/lib/desktop-release.sh"
asset="$(web_package_asset "$package")"

# The draft is the staging run's (stage-desktop.sh, or the train's `draft`
# job), created on the staged commit: join it, never create or move it.
join_desktop_draft
mv "$REPOSITORY_DIR/web/target/taigi-keyboard-$package.zip" "$RELEASE_TEMP_DIR/$asset"
stage_desktop_asset "$RELEASE_TEMP_DIR/$asset"
echo "attached $asset to $DESKTOP_TAG (built from $DESKTOP_SOURCE_COMMIT)"
