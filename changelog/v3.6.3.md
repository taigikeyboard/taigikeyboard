## v3.6.3

Headline release: **TPS (注音) input fixes** — explicit-tone filtering, a tone-9 (ˆ) input path, and the ir [ɨ] vowel after sibilants (自 / 事) now typeable — plus short single-initial input surfaces common single characters again across TL / POJ / TPS, a refreshed shared emoji set, and Android fixes for English-mode autocorrect, key-press vibration, and backspace in rich web editors (Gmail / Google Chat). 顯示羅馬字 now defaults off.

### Shared (iOS + Android)

#### New Features

- **Explicit-tone filtering in TPS.** Typing a tone in TPS now shows only that tone's readings (matching the TL / POJ behaviour shipped in v3.6.0); typing without a tone still shows all tones. (#433)
- **TPS tone-9 (ˆ) input.** The ninth tone finally has an input path — the `9` digit key converts to the ˆ mark when it follows a toneable syllable, so tone-9 words (昨昏 ㄗㄤˆ / 才 ㄘㄞˆ) are reachable. (#434)

#### Bug Fixes

- **TPS ir [ɨ] vowel after sibilants.** Typing ts / tsh / s / j + ir (自 tsīr / 事 / 故事) produced no candidates — the romanization converter mis-segmented the palatalized initial and dropped the row. About 426 ir-family words are now reachable via TPS. (#435)
- **Short input surfaces common single characters again.** Typing a single initial in TL / POJ (e.g. `s`) or TPS (e.g. ㄍ) buried high-frequency single characters (是 / 個) under longer multi-syllable words; the candidate strip now drains the shortest matching keys first, so single characters appear. (#441 / #443)

#### Changes

- **顯示羅馬字 (show romanization) now defaults off.** The 漢羅 (Han-Lo) literal-roman first candidate — added in v3.6.1, made a toggle in v3.6.2 — is now opt-in. Turn it on in 拍字設定 or the in-keyboard quick settings to keep the romanization you type as the first candidate. (#445)
- **Emoji palette refreshed.** Both platforms now read the same shared `taigi-emojis` data set; a few deprecated multi-person family sequences were dropped and newer emoji added. (#436 / #438)

### iOS

#### Bug Fixes

- **Emoji keyboard shows 4 rows again.** The emoji-source swap had pulled a stale layout that fit only 3 rows; the vendored view is back in sync so the palette shows 4 rows. (#440)

### Android

#### Bug Fixes

- **English-mode autocorrect works without a system spell checker.** EN input showed zero candidates on devices whose system `SpellCheckerSession` returns null (common on Samsung One UI); a bundled 30k-word frequency list now drives prefix completion and spelling correction in-app. iOS keeps its `UITextChecker` path (intentional divergence). (#432)
- **Key-press vibration follows the in-app toggle.** With the in-app vibration toggle on but the OS touch-vibration setting off, key presses produced no haptic feedback on some devices. The keyboard now drives a direct `Vibrator`, so the in-app toggle is the authority. (Does not override the system master vibration switch / battery-saver.) (#444)
- **Backspace deletes text in rich web editors.** Backspace over committed text did nothing in WebView / contenteditable hosts (Gmail / Google Chat message boxes), which silently drop a synthetic `KEYCODE_DEL`. Deletion now dispatches by editor capability — the semantic `deleteSurroundingText` (grapheme-aware) for rich editors, raw key events only for `TYPE_NULL` editors. iOS was unaffected (its `deleteBackward()` is already semantic). (#446)

### Engine (Rust shared core)

- **TPS explicit-tone filter.** `span_is_fully_toned_tps` + a TPS arm in `fst_body_for_span` build a verbatim toned `tps:<tps_num>` key (tone-8 scalar normalized U+02D9→U+0307); tones 1 / 4 carry no distinguishing mark and stay all-tones. `normalize_tps_tone8_scalar` extracted as the single char-level source. (#433)
- **TPS tone-9 digit conversion.** `adjust_tone_nine_digit` converts a TPS `9` → ˆ when the buffer ends in a toneable (non-stop) syllable body, running first in `adjust` so the mark feeds `syllabic_nasal_replacement` like a directly-typed tone. (#434)
- **Partial-prefix shortest-first hydration.** `PrefixIndex::lookup_prefix_shortest_first` buckets survivors by matched-key byte length and drains shortest-first into the hydrate cap, so short high-frequency readings are not starved by the FST wire order (`key || 0xFF || rowid`). Initial-only abbreviation surfaces are dropped per-mode via `phonetics::is_tps_initial_only` (TPS) and `phonetics::is_roman_acronym_key` (TL / POJ). Search-axis `lookup_prefix` and the FST are untouched. (#441 / #443)
- Generated proto + xcframework + jniLibs regenerated via `make build`.

### Dictionary

- **TPS ir-vowel converter fix.** The `taigi-converter` submodule now skips a palatalized initial (ending in `i`) when the next char is `r` (the `ir` central-vowel head), with Rust parity in `engine/phonetics/src/tps.rs::to_zhuyin`. `make dict` repopulated `tps_num` / `tps_notone` for the ir rows; `dictionary.fst` grew with the new `tps:` keys and `association.bin` was regenerated. (#435)
- Regenerated via `make dict` at release.

### Documentation

- `docs/architecture/behavioral-invariants.md` — §36 `INVARIANT_KEYPRESS_FEEDBACK_APP_TOGGLE_GATE` (key-press feedback OS-master divergence); §17 TPS explicit-tone filter extension.
- `.claude/rules/android-ime-patterns.md` — §2 committed-text delete-dispatch rule (semantic `InputConnection` API vs raw key events by editor capability). (#446)
- `docs/references/mainstream-ime-comparison.md` — catalogued PhahTaigi_iOS, rime-phah-taibun, and the dictionary-supplement source (all 25 `references/` repos now listed).

### Removed

- Android: `assets/ime/media/emoji/{emoji-test.txt,emoji-test.txt.backup,root.txt}` — replaced by the shared `taigi-emojis` `emoji.json`.
- iOS: vendored `ISEmojiList*.plist` — replaced by `emoji.json` as the single emoji source.

### New Files

- iOS: `Emojis/TaigiEmojiData.swift`; vendored `Vendor/ISEmojiView/**` (was a remote SPM dependency).
- Android: `ime/text/composing/EnglishWordMatcher.kt`, `assets/english_freq.txt`, `tools/build_english_freq.py`; `ime/core/KeyPressVibrator.kt`.
- Engine: `composing/tests/cross_mode_parity.rs`, `lexicon/tests/tlpoj_partial_prefix_abbrev.rs`, `lexicon/tests/tps_partial_prefix_abbrev.rs`.
- Dictionary: `dictionary/tools/gen_dogfood.py`.
