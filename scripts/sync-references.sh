#!/usr/bin/env bash
# Sync the gitignored references/ directory: clone missing reference IME
# repos, fast-forward existing ones to latest. Idempotent; safe to re-run.
#
# Roster source of truth: docs/references/mainstream-ime-comparison.md
# (every directory the comparison doc names, plus the clones it does not
# yet catalogue). Update BOTH when adding a repo. Clones prefer the `taigikeyboard` GitHub org fork; repos
# the org has not forked come from upstream.
#
# Usage:
#   scripts/sync-references.sh              # sync everything
#   scripts/sync-references.sh librime KeSi # sync only the named dirs
#
# Per-repo behaviour:
#   missing            -> clone (mozc: shallow; keyboardkit9.9.0: pinned tag)
#   exists, clean      -> git pull --ff-only (shallow: fetch+reset; pinned:
#                         fetch tags only, checkout stays on the pin)
#   exists, dirty      -> skipped with DIRTY (never clobbers local edits)
#   exists, detached   -> fetch only (treated as a manual pin)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REF_DIR="$REPO_ROOT/references"

# dir|url|mode   mode: "" = plain, shallow, pin:<tag>[:<upstream-url>]
REPOS=(
  # -- taigikeyboard org forks (incl. the two decompiled-APK archives) --
  "azooKey|git@github.com:taigikeyboard/azooKey.git|"
  "CustardKit|git@github.com:taigikeyboard/CustardKit.git|"
  "florisboard|git@github.com:taigikeyboard/florisboard.git|"
  "rime-moetaigi|git@github.com:taigikeyboard/rime-moetaigi.git|"
  "McBopomofo|git@github.com:taigikeyboard/McBopomofo.git|"
  "lexical-models|git@github.com:taigikeyboard/lexical-models.git|"
  "trime|git@github.com:taigikeyboard/trime.git|"
  "rakukan|git@github.com:taigikeyboard/rakukan.git|"
  "PIME|git@github.com:taigikeyboard/PIME.git|"
  "MacishType|git@github.com:taigikeyboard/MacishType.git|"
  "PhahTaigi_iOS|git@github.com:taigikeyboard/PhahTaigi_iOS.git|"
  "rime-phah-taibun|git@github.com:taigikeyboard/rime-phah-taibun.git|"
  "ISEmojiView|git@github.com:taigikeyboard/ISEmojiView.git|"
  "KeSi|git@github.com:taigikeyboard/KeSi.git|"
  "Taigi-Input-method-dictionary-supplement|git@github.com:taigikeyboard/Taigi-Input-method-dictionary-supplement.git|"
  "moe_taigi_apk|git@github.com:taigikeyboard/moe_taigi_apk.git|"
  "aiongtaigi-sushi|git@github.com:taigikeyboard/aiongtaigi-sushi.git|"
  "khiin-rs|git@github.com:taigikeyboard/khiin-rs.git|"
  "mozc|git@github.com:taigikeyboard/mozc.git|shallow"
  # Fork carries no tags; the pin fetches them from upstream KeyboardKit.
  "keyboardkit9.9.0|git@github.com:taigikeyboard/KeyboardKit.git|pin:9.9.0:git@github.com:KeyboardKit/KeyboardKit.git"
  # -- no org fork yet: upstream --
  "librime|git@github.com:rime/librime.git|"
  "librime-predict|git@github.com:rime/librime-predict.git|"
  "azooKey-Desktop|git@github.com:azooKey/azooKey-Desktop.git|"
  "Hamster|git@github.com:imfuxiao/Hamster.git|"
  "vChewing-macOS|git@github.com:vChewing/vChewing-macOS.git|"
  "vChewing-LibVanguard|git@github.com:vChewing/vChewing-LibVanguard.git|"
  "Tekkon|git@github.com:vChewing/Tekkon.git|"
  "azooKey_emoji_dictionary_storage|git@github.com:azooKey/azooKey_emoji_dictionary_storage.git|"
  "KeyKey41-Eten-Tribute|git@github.com:whyren0324/KeyKey41-Eten-Tribute.git|"
  # The offline authority for KeyboardKit (.claude/rules/doc-lookup.md): a
  # published DocC archive, but a real git repo — clone it like any other.
  "KeyboardKit-Documentation|git@github.com:KeyboardKit/KeyboardKit-Documentation.git|"
)

mkdir -p "$REF_DIR"

filter=("$@")
wants() {
  [ ${#filter[@]} -eq 0 ] && return 0
  local want
  for want in "${filter[@]}"; do [ "$want" = "$1" ] && return 0; done
  return 1
}

failed=()
dirty=()
report() { printf '%-8s %s%s\n' "$1" "$2" "${3:+ — $3}"; }

for entry in "${REPOS[@]}"; do
  IFS='|' read -r dir url mode <<<"$entry"
  wants "$dir" || continue
  path="$REF_DIR/$dir"

  if [ ! -d "$path" ]; then
    case "$mode" in
      shallow)
        git clone --quiet --depth 1 "$url" "$path" \
          && report CLONED "$dir" "shallow" || { report FAIL "$dir" "clone"; failed+=("$dir"); }
        ;;
      pin:*)
        tag="${mode#pin:}"; tag="${tag%%:*}"
        upstream="${mode#pin:*:}"
        if git clone --quiet "$url" "$path" \
          && git -C "$path" remote add upstream "$upstream" \
          && git -C "$path" fetch --quiet --tags upstream \
          && git -C "$path" checkout --quiet "$tag"; then
          report CLONED "$dir" "pinned @ $tag"
        else
          report FAIL "$dir" "clone/pin"; failed+=("$dir")
        fi
        ;;
      *)
        git clone --quiet "$url" "$path" \
          && report CLONED "$dir" || { report FAIL "$dir" "clone"; failed+=("$dir"); }
        ;;
    esac
    continue
  fi

  if [ -n "$(git -C "$path" status --porcelain 2>/dev/null)" ]; then
    report DIRTY "$dir" "local changes, skipped"
    dirty+=("$dir")
    continue
  fi

  case "$mode" in
    pin:*)
      # Stay on the pin; just refresh upstream tags for a future re-pin.
      git -C "$path" fetch --quiet --tags upstream 2>/dev/null || true
      report PINNED "$dir" "@ $(git -C "$path" describe --tags --always)"
      ;;
    shallow)
      before="$(git -C "$path" rev-parse HEAD)"
      if git -C "$path" fetch --quiet --depth 1 origin \
        && git -C "$path" reset --quiet --hard "$(git -C "$path" rev-parse FETCH_HEAD)"; then
        after="$(git -C "$path" rev-parse HEAD)"
        [ "$before" = "$after" ] && report OK "$dir" "up to date" \
          || report UPDATED "$dir" "${before:0:8}..${after:0:8}"
      else
        report FAIL "$dir" "shallow fetch"; failed+=("$dir")
      fi
      ;;
    *)
      if ! git -C "$path" symbolic-ref --quiet HEAD >/dev/null; then
        git -C "$path" fetch --quiet origin || true
        report PINNED "$dir" "detached HEAD, fetch only"
        continue
      fi
      before="$(git -C "$path" rev-parse HEAD)"
      if git -C "$path" pull --quiet --ff-only 2>/dev/null; then
        after="$(git -C "$path" rev-parse HEAD)"
        [ "$before" = "$after" ] && report OK "$dir" "up to date" \
          || report UPDATED "$dir" "${before:0:8}..${after:0:8}"
      else
        report FAIL "$dir" "pull --ff-only (diverged or offline?)"; failed+=("$dir")
      fi
      ;;
  esac
done

known() {
  local e
  for e in "${REPOS[@]}"; do [ "${e%%|*}" = "$1" ] && return 0; done
  return 1
}
for want in "${filter[@]:-}"; do
  [ -n "$want" ] && ! known "$want" && echo "WARN: unknown repo name '$want' (see roster in this script)" >&2
done

if [ ${#dirty[@]} -gt 0 ]; then
  echo "DIRTY (skipped, commit or discard the local changes to sync): ${dirty[*]}" >&2
fi

if [ ${#failed[@]} -gt 0 ]; then
  echo "FAILED: ${failed[*]}" >&2
  exit 1
fi
