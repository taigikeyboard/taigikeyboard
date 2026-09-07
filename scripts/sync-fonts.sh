#!/usr/bin/env bash
# Distributes the candidate-window typefaces to the platforms that package them
# from a committed copy, and checks the copies still match.
#
# The fonts are external assets, not build output, so nothing regenerates them
# the way `make dict` regenerates the dictionary. `ios/Resources/Fonts/` is the
# designated source; macOS and Windows keep their own copies so their packaging
# scripts read from their own platform directory.
#
# iOS reads the source directory itself. Android is deliberately out of scope:
# its copies live under `res/font/` with Android's lowercase resource naming,
# and two of its four files differ from the source in three bytes of `head`
# table checksum — matching them byte for byte would change what Android ships.
#
#   scripts/sync-fonts.sh           # copy source -> platform copies
#   scripts/sync-fonts.sh --check   # verify, change nothing, fail on drift
set -euo pipefail

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="$REPOSITORY_DIR/ios/Resources/Fonts"
DESTINATIONS=(
    "$REPOSITORY_DIR/macos/Resources/Fonts"
    "$REPOSITORY_DIR/windows/resources/Fonts"
)

check_only=false
case "${1:-}" in
    --check) check_only=true ;;
    "") ;;
    *) echo "sync-fonts: unknown argument '$1' — expected --check or nothing" >&2; exit 2 ;;
esac

# Enumerated exactly the way the two packaging scripts do — top-level `*.ttf`
# and `*.otf`, no `find`, no type test — so a face this script considers absent
# can never be one `macos/scripts/bundle-app.sh` or `windows/scripts/release-app.sh`
# would package. Unmatched globs stay literal, hence the existence test.
list_faces() {
    local directory="$1" candidate
    for candidate in "$directory"/*.ttf "$directory"/*.otf; do
        [[ -e "$candidate" ]] && basename "$candidate"
    done | sort
}

[[ -d "$SOURCE_DIR" ]] || {
    echo "sync-fonts: missing source directory $SOURCE_DIR" >&2
    exit 1
}

source_faces="$(list_faces "$SOURCE_DIR")"
[[ -n "$source_faces" ]] || {
    echo "sync-fonts: no fonts in $SOURCE_DIR" >&2
    exit 1
}

# An empty source face would be skipped by the packaging scripts' `-s` test and
# silently drop a typeface out of the picker, so refuse to distribute one.
while IFS= read -r face; do
    [[ -s "$SOURCE_DIR/$face" ]] || {
        echo "sync-fonts: source face $face is empty" >&2
        exit 1
    }
done <<< "$source_faces"

status=0
for destination in "${DESTINATIONS[@]}"; do
    relative="${destination#"$REPOSITORY_DIR/"}"

    if ! $check_only; then
        mkdir -p "$destination"
        # Copy through a temporary name and rename over the target, so an
        # interrupted run leaves the previous copy intact rather than a
        # half-written face the packaging scripts would happily ship.
        while IFS= read -r face; do
            cp "$SOURCE_DIR/$face" "$destination/.$face.tmp"
            mv -f "$destination/.$face.tmp" "$destination/$face"
        done <<< "$source_faces"
        # Only once every face is in place: a face dropped from the source has
        # to disappear from the copies too.
        while IFS= read -r face; do
            grep -qxF "$face" <<< "$source_faces" || rm -f "$destination/$face"
        done <<< "$(list_faces "$destination")"
        echo "synced $relative"
        continue
    fi

    if [[ ! -d "$destination" ]]; then
        echo "sync-fonts: missing directory $relative" >&2
        status=1
        continue
    fi

    if [[ "$(list_faces "$destination")" != "$source_faces" ]]; then
        echo "sync-fonts: $relative does not hold the same faces as ${SOURCE_DIR#"$REPOSITORY_DIR/"}" >&2
        diff <(printf '%s\n' "$source_faces") <(list_faces "$destination") >&2 || true
        status=1
        continue
    fi

    while IFS= read -r face; do
        if ! cmp -s "$SOURCE_DIR/$face" "$destination/$face"; then
            echo "sync-fonts: $relative/$face differs from the source" >&2
            status=1
        fi
    done <<< "$source_faces"

    [[ $status -eq 0 ]] && echo "ok $relative"
done

if $check_only && [[ $status -ne 0 ]]; then
    echo "sync-fonts: run 'make fonts' and commit the result" >&2
fi
exit $status
