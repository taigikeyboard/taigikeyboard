#!/bin/bash
#
# Publish dictionary packages to GitHub Release
#
# Usage:
#   ./09_publish_release.sh                    # Create release with all packages
#   ./09_publish_release.sh --repo owner/repo  # Specify repo (default: taigikeyboard/dictionary)
#
# Prerequisites:
#   - gh CLI installed and authenticated
#   - output/packages/ contains built packages (run 08_split_packages.py first)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BASE_DIR="$(dirname "$SCRIPT_DIR")"
PACKAGES_DIR="$BASE_DIR/output/packages"

# Default repo
REPO="taigikeyboard/dictionary"

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo)
            REPO="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=================================================="
echo "09_publish_release"
echo "=================================================="
echo "Repo:     $REPO"
echo "Packages: $PACKAGES_DIR"
echo ""

# Check prerequisites
if ! command -v gh &> /dev/null; then
    echo -e "${RED}Error: gh CLI not found. Install: https://cli.github.com/${NC}"
    exit 1
fi

if [ ! -d "$PACKAGES_DIR" ]; then
    echo -e "${RED}Error: Packages directory not found: $PACKAGES_DIR${NC}"
    echo "  Run 08_split_packages.py first."
    exit 1
fi

MANIFEST="$PACKAGES_DIR/remote-manifest.json"
if [ ! -f "$MANIFEST" ]; then
    echo -e "${RED}Error: remote-manifest.json not found${NC}"
    exit 1
fi

# Extract version from manifest
VERSION=$(python3 -c "import json; print(json.load(open('$MANIFEST'))['version'])")
TAG="dict-v${VERSION}"

echo "Version:  $VERSION"
echo "Tag:      $TAG"
echo ""

# Collect files to upload
FILES=("$MANIFEST")
for zip in "$PACKAGES_DIR"/*.zip; do
    if [ -f "$zip" ]; then
        FILES+=("$zip")
        echo "  $(basename "$zip") ($(du -h "$zip" | cut -f1))"
    fi
done

echo ""
echo "Total files: ${#FILES[@]}"
echo ""

# Create or update release
if gh release view "$TAG" --repo "$REPO" &> /dev/null; then
    echo "Release $TAG exists, updating..."
    # Delete existing assets and re-upload
    gh release delete-asset "$TAG" --repo "$REPO" --yes remote-manifest.json 2>/dev/null || true
    for zip in "$PACKAGES_DIR"/*.zip; do
        if [ -f "$zip" ]; then
            gh release delete-asset "$TAG" --repo "$REPO" --yes "$(basename "$zip")" 2>/dev/null || true
        fi
    done
    gh release upload "$TAG" --repo "$REPO" "${FILES[@]}"
else
    echo "Creating release $TAG..."
    gh release create "$TAG" \
        --repo "$REPO" \
        --title "Dictionary $VERSION" \
        --notes "Dictionary packages built on $(date '+%Y-%m-%d %H:%M:%S')" \
        "${FILES[@]}"
fi

echo ""
echo -e "${GREEN}Published to: https://github.com/$REPO/releases/tag/$TAG${NC}"
