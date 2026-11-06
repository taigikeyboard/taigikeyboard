#!/usr/bin/env bash
# Cuts every font in <assets/fonts/font> to the one unicode-range fonts.css gives
# them, as woff2. Every face repeats that range (fonts.css says why), and this reads it
# from there so the two cannot drift. A whole typeface is 5 to 15 MB; these are
# build output, like the bindings, and never committed.
#   scripts/subset-fonts.sh <assets/fonts/font> <output directory>
set -euo pipefail

source_dir="$1"
output_dir="$2"
stylesheet="$(dirname "$0")/../src/fonts.css"

# One line per distinct range, each as "U+3000-303F,U+3100-312F,…".
ranges="$(tr -d '\n' <"$stylesheet" | { grep -o 'unicode-range:[^;]*' || true; } |
  sed 's/unicode-range://; s/ //g' | sort -u)"
if [[ "$ranges" != U+* || "$ranges" == *$'\n'* ]]; then
  echo "unicode-range in fonts.css is missing or differs between faces: $ranges" >&2
  exit 1
fi

mkdir -p "$output_dir"
pids=()
for path in "$source_dir"/*; do
  name="$(basename "$path")"
  pyftsubset "$path" --unicodes="$ranges" --layout-features='*' \
    --flavor=woff2 --output-file="$output_dir/${name%.*}.woff2" &
  pids+=("$!")
done
# One `wait` per job: a bare `wait` would hide a failure.
for pid in "${pids[@]}"; do
  wait "$pid"
done
