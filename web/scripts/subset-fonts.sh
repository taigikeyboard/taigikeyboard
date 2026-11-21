#!/usr/bin/env bash
# Cuts every font in <assets/fonts/font> to the unicode-range fonts.css gives
# them, as woff2. Every face repeats one range and its "Hanji" twin a tail of it
# (fonts.css says why); this reads the range from there so the two cannot drift.
# A whole typeface is 5 to 15 MB; these are build output, like the bindings, and
# never committed.
#   scripts/subset-fonts.sh <assets/fonts/font> <output directory>
set -euo pipefail

source_dir="$1"
output_dir="$2"
stylesheet="$(dirname "$0")/../src/fonts.css"

# One line per distinct range, each as "U+3000-303F,U+3100-312F,…". The longest is
# the faces' own range, which every other one must end.
ranges="$(tr -d '\n' <"$stylesheet" | { grep -o 'unicode-range:[^;]*' || true; } |
  sed 's/unicode-range://; s/ //g' | sort -u)"
full="$(awk 'length > n { n = length; r = $0 } END { print r }' <<<"$ranges")"
if [[ "$full" != U+* ]]; then
  echo "unicode-range in fonts.css is missing" >&2
  exit 1
fi
while IFS= read -r range; do
  if [[ "$full" != *",$range" && "$full" != "$range" ]]; then
    echo "unicode-range in fonts.css differs between faces: $range" >&2
    exit 1
  fi
done <<<"$ranges"

mkdir -p "$output_dir"
pids=()
for path in "$source_dir"/*; do
  name="$(basename "$path")"
  pyftsubset "$path" --unicodes="$full" --layout-features='*' \
    --flavor=woff2 --output-file="$output_dir/${name%.*}.woff2" &
  pids+=("$!")
done
# One `wait` per job: a bare `wait` would hide a failure.
for pid in "${pids[@]}"; do
  wait "$pid"
done
