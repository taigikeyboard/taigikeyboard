# 漢羅齊出 (Han-Lo together) candidate mode — research

> **Type**: Report (dated snapshot, frozen) — research only, **nothing implemented**
> **Keywords**: `漢羅齊出`, `hanlo`, `isTranslateSwapped`, `outputBothScripts`, `CandidateCellContent`, `CandidateCellHelper`, `SmartbarCandidateStrip`, `§42`, `§34`, `dedupe_by_roman_hanji_span`
> **Date**: 2026-08-30
> **Platforms named by USER**: iOS, Android, macOS (Windows not named)
> **Status**: USER 2026-08-30 — 「目前還不打算 implement，先做 research 撰寫文件，供未來參考」. No release, no version, no branch assigned.

---

## 1. Feature statement (USER, 2026-08-30, verbatim)

> For tl/poj, not tps
> 漢字/羅馬字混合模式，當這個設定開啟後，不需要 swap translate button，不需要分成 title/subtitle，漢字和羅馬字會一併出現。預設設定關閉，設定名稱：漢羅齊出
>
> 可能需要注意的：可能會有重複字；可能影響 Custom dictionary, word association, 詞頻紀錄

Read as four requirements plus two risk flags:

| # | Requirement | Reading |
|---|---|---|
| R1 | TL / POJ only, never TPS | TPS stays hanji-only (`effectiveSwapped` forced, §5 display-dedupe by hanji). |
| R2 | 文/A swap key becomes unnecessary | Either hidden or inert while the mode is on. |
| R3 | No title / subtitle split | Cell shows ONE label carrying both scripts, e.g. `台語 tâi-gí`, instead of a primary + smaller secondary. |
| R4 | Both scripts "appear together" | Certain for the **cell**. Whether the **committed document text** also carries both is not stated — see §8 Q1. |
| F1 | 可能會有重複字 | Analysed in §6. |
| F2 | 可能影響 自訂詞 / 詞關聯 / 詞頻 | Analysed in §7. |

Default OFF. Setting name 漢羅齊出.

## 2. Terminology — three things already called "漢羅" / "both scripts"

The codebase and the reference IMEs use "漢羅" for three different mechanisms. The new mode is a **fourth**; naming it precisely avoids re-implementing one of the others.

| Term | Mechanism | Where | Relation to 漢羅齊出 |
|---|---|---|---|
| **§42 cell shows both scripts** (`INVARIANT_CANDIDATE_CELL_SHOWS_BOTH_SCRIPTS`) | Every cell already renders hanji AND roman, as **primary + secondary** visual roles; swap decides which leads. `behavioral-invariants.md:1026-1040` | all three platforms | 漢羅齊出 changes the *arrangement* (one label, R3) — the invariant's "never a formatted string" clause (`:1033`) would need an explicit carve-out. |
| **括號標註** (`outputBothScripts`, "Annotate in Brackets") | **Commit-side** rendering: document gets `hit (彼)` / `彼 (hit)`. `i18n/settings.json:49-53`. Cell untouched. | iOS + Android live; macOS setting exists but UI retired, value wiped each launch (`RetiredSettingsCleanup.swift:111`, pinned false per `AutoSpacePolicy.swift:13`). | Sibling. If 漢羅齊出 also commits both scripts, the two overlap — §8 Q1/Q4. |
| **§34 literal-roman candidate** (顯示原本羅馬字候選, "漢羅 fast input") | Index-0 roman-only candidate so 漢羅 *mixed-script writers* commit romanization in one tap even in hanji-first mode. `behavioral-invariants.md:843-866`. | engine, TL/POJ | Orthogonal; stays. Under 漢羅齊出 its cell reads `tâi` (one script) next to `台 tâi` — see §6 (b). |
| **漢羅混寫** (per-word Han-vs-Lo choice by LKK rules) | Output text mixes hanji words and roman words by rule — `我 beh 去 tshit-thô`. `references/rime-phah-taibun/lua/phah_taibun_filter.lua:79-215`. | reference IME only | **Different feature** (automatic script selection per word). Not what USER asked. Listed in §9 as deliberately not adopted. |

## 3. Current state — per platform (grounded in code)

### 3.1 Swap / 文/A toggle

| | iOS | Android | macOS |
|---|---|---|---|
| Setting key | `SharedSettings.swift:44` `isTranslateSwapped` (default false) | `PreferenceDataStore.kt:46` `keyboard__is_translate_swapped`, `PrefHelper.kt:318` | `SettingsStore.swift:46-49` `"isTranslateSwapped"` (iOS-aligned spelling) |
| Key / UI | `KeyDef.swift:22` `.translate`; bottom row of every Taigi layout `TaigiLayouts.swift:32,46,60,117,131,145,159,201,240`; icon `ButtonImageProvider.swift:37-41`; tap → `ActionHandler+CustomActions.swift:11-21` → `KeyboardContext+Translate.swift:11-25` | `KeyCode.kt:67` `TRANSLATE = -230` in layout JSON (`moe2_halfwidth.json:35-37` …); icon `KeyContent.kt:440`, active bg `:620`; tap → `TextInputKeyHandler.kt:167-169` → `SmartbarManager.kt:521-542`; overlay control panel button `CandidateOverlayContent.kt:220` (`showTranslate = !isTPSLayout`) | **No key, no menu item** — shortcut action only: `ShortcutActions.swift:66,79,85,114` → `TaigiInputController.swift:420-427` (`rerenderCandidatesForDisplayChange`) |
| Side effects of the flag | Full-width char keys `LayoutConverter.swift:43`; confirm-key label 選 vs suán `ButtonTextProvider.swift:194-215` | Full/half-width layout variant `LayoutManager.kt:265-300,323`; `invalidateKeysByCode(TRANSLATE, VIEW_NUMERIC_ADVANCED)` | Full-width punctuation gate `TaigiInputController.swift:749-751`; auto-space gate `:762-772` |
| `effectiveSwapped` | `ComposingManager.swift:181-188` (`isTranslateSwapped \|\| inputMode == .tps`) for engine; commit path recomputes from **layout type** `ActionHandler+Suggestions.swift:81-82,140-141`; auto-space `ActionHandler+KeyActions.swift:150-156` | `CandidateClickHandler.kt:86-89, 225-228, 342-345` (`isTpsLayout \|\| cached`), 4th copy `TextInputKeyHandler.kt:608-613` | No TPS → `settings.isTranslateSwapped` **is** effective |

Every one of those `effectiveSwapped` sites is a place 漢羅齊出 must take a position on (§5).

### 3.2 Candidate cell rendering (title / subtitle)

| | iOS | Android | macOS |
|---|---|---|---|
| Chooser | `CandidateCellHelper.swift:24-57` `displayTitle` / `displaySubtitle` — swapped ⇒ hanji leads | `SmartbarCandidateStrip.kt:198-232`; overlay `CandidateOverlayContent.kt:358-364` — same 4-arm `when` (`hanzi empty → roman only` / TPS / swapped / else) | `CandidateCellContent.swift:43-51` `text` + optional `annotation` |
| Layout | `CandidateButtonView.swift:64-79` `VStack` Text(title) over Text(subtitle); grid cell `ExpandedCandidateGridCell.swift:62-81` (invisible `" "` spacer keeps two-line height) | Column of two `BasicText` `:265-289`; overlay two `Text` `maxLines=1` `:366-386` | `CandidateItemView.swift:32-34,137-140` — `.inline` (vertical layout, annotation column aligned on one x) or `.stacked` (horizontal / expandable), `CandidateCellArrangement.swift:11-16` |
| Width | Expanded overlay only: `CandidateCellHelper.swift:99-125` measures **both** at own font, `max + 20`, floor 44 ("一律兩者都量，避免 translate toggle 時佈局 reflow") | Overlay `CandidateOverlayContent.kt:536-553` `max(roman, hanzi) + padding`; strip intrinsic | `CandidateMetrics.swift:337-348` inline = text + annotation, stacked = max |
| Same-string guard | `subtitle != displayTitle` `CandidateButtonView.swift:72` | strip only `:233`; overlay none | — |
| Fonts | `CandidateTheme.swift:27-28,58-95` primary 19/21/19/23, secondary 14/16/14/17 × scale | `computeCandidateFontSizes` `:235-246`, `PRIMARY_TEXT_SIZE_SP` / `SUBTITLE_TEXT_SIZE_SP` | `CandidateMetrics.swift:115-120,224`, font picker `CandidateFontChoice.swift:22-49` |
| Index-0 keycap highlight (S6) | `CandidateSuggestionsRow.swift:81` → `CandidateViewStyle.swift:151-193` | `SmartbarCandidateStrip.kt:190-196`; overlay `:316-347` | n/a (macOS uses selection + slot key `CandidateIndexLabel.swift:26-31`) |
| Preview | iOS `KeyboardPreviewPanel.swift:56-66` three mock candidates `mī-tê/麵茶 …`; `ThemePreviewEnvironment.swift:57-58` reads live swap | Android `KeyboardPreviewPanel.kt:118` hardcodes `isTranslateSwapped = false`, **no candidate strip in preview** | — |

Width is the structural point: today the widest of the two scripts sets the cell; a single `漢字 羅馬字` label is **sum + gap**, so every strip / overlay / panel gets wider cells and fewer per row (macOS inline arrangement already pays sum + gap — `CandidateMetrics.swift:341`).

### 3.3 Commit formatter (what the document receives)

All three platforms hold the same 3-arm formula, in this order:

```
hanji absent/empty            → roman
outputBothScripts && hanji    → swapped ? "hanji (roman)" : "roman (hanji)"
swapped && hanji              → hanji
else                          → roman
```

- iOS `ActionHandler+Suggestions.swift:218-232` `formatOutputText` (+ TPS `bracketRoman` `:219-221`); `parseRomanAndHanzi` `:185-214` undoes `CandidateCellHelper.suggestionToHandle`'s pre-swap (`:65-91`).
- Android `CandidateClickHandler.kt:99-117`, `:237-255`, `:346-370` — three copies (legacy / overlay / continuous).
- macOS `CandidateDocumentText.swift:23-38`; plus **Space commits the other script** (#610): `alternateText` `:70-75` is read **off the cell's annotation**, deliberately ignores `outputBothScripts` (`:76-79`); a one-script candidate → `.ignored` (`ComposingManager.swift:371-374`).

Engine receives the formatted string only as `CommitContinuous.display_text`; identity travels separately (`canonical_text`, `association_tl`) — iOS `:99-105`, Android `:376-384`, macOS `ComposingManager.swift:363-390`.

### 3.4 Auto-space and other gates that read the swap flag

Predicate everywhere is `!effectiveSwapped || outputBothScripts` (= "the document is roman-ish, wants a word space"):

- engine `composing/src/api.rs:165-169` `continuous_word_space` — the **only** engine reader of `output_both_scripts`; feeds `nailed_prefix` / `combined_display` (`:203, 334-345`) = composing-buffer join + hard-finalize write (`continuous-commit-and-display.md:70`).
- iOS `ActionHandler+Suggestions.swift:129-133,162-166`, `ActionHandler+KeyActions.swift:150-156` (S10 punctuation swap).
- Android `CandidateClickHandler.kt:431-443`, `TextInputKeyHandler.kt:606-614`.
- macOS `AutoSpacePolicy.swift:47-55` (`.primary: !swapped || both` / `.alternate: swapped`).
- engine nextword: `decide.rs:93` (Enter-commits-raw skipped when swapped), `filter.rs:245` (`!swapped && roman.is_empty() → drop prediction`), `filter.rs:238-260` `shape_prediction` `text` = roman-if-present else hanzi, `subtitle` = the other.
- macOS full-width punctuation `TaigiInputController.swift:749-751` (hanji output only, #600).

### 3.5 Existing `outputBothScripts` wiring = the template for a new toggle

| Layer | iOS | Android | macOS |
|---|---|---|---|
| i18n | `i18n/settings.json:49-53` → `make i18n` → `StringKey.swift:186` | → `StringKey.kt:196`, `strings_i18n.xml:189` | → `StringKey.swift:155` |
| Store | `SharedSettings.swift:45,270-273,652` | `PreferenceDataStore.kt:47`, `PrefHelper.kt:320,573-578` | `SettingsStore.swift:50-53`, `EngineSettings.swift:31,81` |
| Contract | `EngineSettings.swift:26-33` (CROSS-PLATFORM INVARIANT comment) | `EngineSettings.kt:54` | `EngineSettings.swift:21-75` |
| Host UI | `SettingsTab.swift:23,60,126-134,344` (`featureSummary("hanloDesign")` info) | `InputSettingsScreen.kt:83,195-203` (`FEATURE_ID_HANLO_DESIGN`) | pane enum `SettingsSplitView.swift:13-42`; `GeneralSettingsView.swift:33-37,79` `@AppStorage` pattern |
| In-keyboard UI | `SettingsSelectionOverlay.swift:18,47,77-79,129`, icon `SettingsIcons.swift:11` | `SettingsOverlayContent.kt:58,97-110`, icon `SettingsIcons.kt:21` | — |
| Engine | `RustEngineBridge+Composing.swift:631-641` `continuousAppConfig` | `RustEngineBridge.kt:1368-1377`, `ComposingManager.kt:923-928` spacing struct | `RustEngineBridge.swift:170-187` |
| Proto | `envelope.proto:70-87` `AppConfig.output_both_scripts = 8` | | |

Note: `featureSummary("hanloDesign")` / `FEATURE_ID_HANLO_DESIGN` — the in-app feature explainer already uses "hanlo" for 括號標註. A new 漢羅齊出 entry in `content/` must not collide with that id.

## 4. Engine facts that bound the design

- **Candidate carries both scripts already.** `CandidateMessage` `composing.proto:363-389`: `roman` (display form, POJ-rendered / recased), `optional hanji` (absent = TAILO roman-only), `display_text = hanji ?? roman` (commit / freq key), `canonical_tl` (identity), `mode ∈ {HANT, TAILO, MIXED}` (`lexicon/src/continuous.rs:212,227` — MIXED = roman letters inside the hanji field, e.g. `台BAR`, MOE `VT_MIXED`). **No engine change is needed to render both** — §42 says so (`behavioral-invariants.md:1038`).
- **Swap is not an engine concept** except three readers (§3.4). `lexicon` / `ranking` / candidate construction never look at `is_translate_swapped`.
- **Dedupe key is `(roman, hanji, span)`** — `dedupe_by_roman_hanji_span` `lexicon/src/continuous.rs:2249-2279`; rationale `:889-905` keeps same-hanji-different-roman AND same-roman-different-hanji (Core Principle #7). Post-render pass `composing/src/continuous.rs:294-299` on the rendered roman. TPS-only display dedupe by `(hanji, span)` `:360` — **TPS only, never TL/POJ** (§5 two-tier, `behavioral-invariants.md:132-155`).
- **§34 literal guard**: `composing/src/dispatch.rs:288-296` removes a dict candidate only when `hanji.is_none() && roman == literal.roman`; `台/tâi` is kept beside bare `tâi` on purpose.
- **No engine "hanji + roman in one string" exists** — `combined_display` (`api.rs:334-345`) is nailed-prefix + tail; the bracket form is platform-only (§3.3).
- **Custom dictionary**: SQL `roman NOT NULL, hanzi NOT NULL` (`custom-dictionary.md:39-51`) but empty hanzi allowed → sent as proto-absent `hanji` (iOS `ComposingManager.swift:442-460`, Android `:663-706`, macOS `:309-317`) → `custom_entry_to_candidate` `lexicon/src/continuous.rs:2147-2190` renders TAILO with `display_text = canonical TL`.

## 5. What 漢羅齊出 ON has to decide, site by site

Because `effectiveSwapped` is read in ~15 places, the cleanest framing is: **漢羅齊出 is a third value of "which script leads", not a fourth boolean beside `isTranslateSwapped`.** Today the state space is `{roman-first, hanji-first} × {plain, 括號標註}`; the new mode adds a display arrangement AND removes the swap axis from the user's reach (R2).

| Site | Today (swapped / not) | Under 漢羅齊出 ON — forced position |
|---|---|---|
| Cell chooser (§3.2) | title/subtitle | one label. Order (`漢字 羅馬字` vs `羅馬字 漢字`) → §8 Q2 |
| Cell width | max(scripts) | sum + gap; overlay packers (`CandidateRowLayout`, `ExpandedCandidateRowLayout`, `CandidateMetrics`) re-measure. iOS/Android comment "swap never reflows" no longer the only invariant to keep. |
| 文/A key | visible (hidden only under TPS) | hidden or inert (R2). Android overlay `showTranslate` already has the TPS precedent `CandidateOverlayContent.kt:220`; iOS layouts hardcode `.translate` in every row — hiding means a layout-level filter, inert means `handleTranslateToggle` no-op. macOS: shortcut action → no-op or leave. |
| Commit formatter (§3.3) | 3 arms | new arm; string → §8 Q1 |
| `continuous_word_space` (engine) | `!swapped \|\| both` | must be `true` iff the committed string is roman-ish. If Q1 = hanji-only → `false`; if Q1 carries roman → `true`. Either add `AppConfig` field or map onto existing two flags (`is_translate_swapped` + `output_both_scripts`) platform-side. |
| Auto-space (4 sites) | same predicate | same answer as above; keep single-sourced. |
| S10 punctuation swap | same predicate | same. |
| macOS Space = alternate script (#610) | annotation → alternate text | there is no annotation any more. Space → ? (§8 Q5) |
| macOS full-width punctuation (#600) | hanji output only | depends on Q1. |
| nextword `shape_prediction` (`filter.rs:238-260`) | `text`/`subtitle` | prediction cells are rendered by the same cell code → same one-label form; `filter.rs:245` drop-rule reads swapped — decide what `is_translate_swapped` is sent as. |
| nextword `decide.rs:93` Enter-commits-raw | skipped when swapped | if 漢羅齊出 maps to swapped=true, Enter raw-commit is lost → likely want swapped=false semantics here. |
| Full-width keycaps (iOS `LayoutConverter.swift:43`, Android `LayoutManager.kt:265-300`) | follow swap | pick one (probably half-width — roman is being written). |
| 選 vs suán confirm label (iOS `ButtonTextProvider.swift:194-215`) | follow swap | pick one. |
| TPS layout while setting ON (R1) | — | setting ignored; TPS path unchanged (all TPS branches already precede the swap arms). |
| Settings preview (iOS mock candidates) | title/subtitle | must render one-label too; Android preview has no strip. |

## 6. F1 — "可能會有重複字": where duplicates can and cannot come from

| Case | Today | Under 漢羅齊出 | Verdict |
|---|---|---|---|
| (a) same roman, different hanji (`tsia̍h` → 食 / 𤆬 / …) | separate rows, differ by subtitle | separate rows `食 tsia̍h` / `𤆬 tsia̍h` — **more** distinguishable | not a duplicate; improved |
| (b) §34 literal `tâi` vs dict `台 tâi` | `tâi` (single line) next to `tâi / 台` | `tâi` next to `台 tâi` | not a duplicate; engine keeps both on purpose (`dispatch.rs:288-296`). Visual proximity is higher because the roman now sits in the same label — cosmetic only. |
| (c) same hanji, different roman (重 tîng / 重 tāng) | two rows both titled 重 in swapped mode | `重 tîng` / `重 tāng` | not a duplicate; **this is the case 漢羅齊出 fixes** — today hanji-first mode shows two identical primaries. |
| (d) custom entry == dict row `(roman, hanji)` | engine dedupe, custom wins (`continuous.rs:855-905`) | unchanged | none |
| (e) custom POJ-form entry rendering same as dict row | `dedupe_rendered_continuous` | unchanged | none |
| (f) MIXED rows (`台BAR`-style, hanji field already contains roman letters) | `台BAR / tâi-bà` | `台BAR tâi-bà` — roman letters appear twice in one label | **real visual doubling**, dictionary-inherent; count how many MIXED rows exist before deciding (`derive_mode` MIXED) |
| (g) hanji == roman string | impossible for real words | — | the `subtitle != title` guards become moot |
| (h) TPS `(hanji, span)` display dedupe | TPS only | R1 excludes TPS | none |
| (i) **commit-side** doubling | `hit (彼)` only under 括號標註 | if Q1 = "both scripts into the document" and 括號標註 is also ON → `彼 hit (彼)`-style triple | must define precedence (§8 Q4) |
| (j) NextWord prediction rows | `text`/`subtitle` from `shape_prediction` | one label like any cell | none, provided both fields are still populated (`filter.rs:238-260` does) |

Net: no *new* engine-level duplicate. The one visual doubling is (f) MIXED dictionary rows; the one semantic doubling is (i) and it is a settings-precedence question, not a data one.

## 7. F2 — 自訂詞 / 詞關聯 / 詞頻 impact

Every store is keyed on the **canonical** pair, never on the document rendering — this was the whole point of R2/R5/§24/§25/§28:

| Store | Key written | Source string on commit | Platform sites |
|---|---|---|---|
| 詞頻 `user_frequency` | `(word, tl)` = `(display_text, canonical_tl)` | engine sidechannel `display_text` (= hanji ?? roman), never `formatOutputText` output | iOS `ActionHandler+Suggestions.swift:117-119,153-155` → `UserFrequencyService.swift:31-40`; Android `CandidateClickHandler.kt:157-165,269-276,394-399` → `UserFrequencyService.kt:208-232`; macOS `ComposingManager.swift:416-428` (comment `:411-415` says exactly this: "recording under the document rendering instead would key the row on a string that changes with the 漢羅 settings") |
| 詞關聯 `user_association` | `(prev_word, prev_tl, next_word, next_tl)` | `canonical_text` + `association_tl` on `CommitContinuous` (`transition.rs:820-880`); legacy path `displayText` + parsed roman (iOS `:172-175`, Android `NextWordHandler.kt:158-193` — `committedText` used **only** for sentence-end punctuation reset `:168-179`) | engine effects |
| 自訂詞 | stored `(roman, hanzi)` raw; query key derived from raw input (`CustomDictionaryDerivation`) | not touched by display mode | §4 |

Conclusion: **no schema or key change is required**, on condition that the new commit arm keeps sending `canonical_text` / `association_tl` exactly as the existing arms do (it is the same call). Two residual points:

1. `is_translate_swapped` also gates *whether* NextWord records / offers (`decide.rs:93`, `filter.rs:245`, macOS `RustEngineBridge+NextWord.swift:117-124` "swap suppresses recording for raw-romanization commits"). Whatever value 漢羅齊出 sends there changes learning behaviour for Enter-raw commits — decide deliberately (§5 rows nextword).
2. Roman-only custom entries (empty hanzi) render as one-script cells (`tâi` alone) — same as §34 literal; fine, but the S13 dogfood row should include one.

## 8. Open questions for USER (batched — answer once)

| # | Question | Why it matters | Options seen |
|---|---|---|---|
| Q1 | **What does a tap commit?** | Drives `continuous_word_space`, auto-space, S10, macOS #600/#610, 括號標註 precedence, and whether an engine `AppConfig` field is needed at all. | (a) hanji only (cell is the annotation, document stays clean; = today's swapped) · (b) `漢字 羅馬字` space-joined (the label as typed) · (c) reuse 括號標註 form `漢字 (羅馬字)` · (d) roman only |
| Q2 | Label order in the cell: `漢字 羅馬字` or `羅馬字 漢字`? Fixed or follows the now-hidden `isTranslateSwapped`? | Cell code; §42 carve-out wording; PhahTaigi / khiin both put hanji as the value and roman as hint when in hanji output mode. | fixed hanji-first · fixed roman-first · inherit swap |
| Q3 | 文/A key: **hidden** (layout filter, like TPS) or **inert** (visible, does nothing)? | iOS layouts hardcode the key in 9 rows; hidden = layout-level change + width redistribution; inert = one-line guard. | hidden · inert |
| Q4 | Precedence with 括號標註 when both ON | Avoid `彼 hit (彼)`. | 漢羅齊出 disables 括號標註 UI · 括號標註 wins · mutually exclusive picker |
| Q5 | macOS Space (= commit alternate script, #610) under 漢羅齊出 | No annotation exists to read; today `.ignored` for one-script cells. | Space inert · Space commits "the other" per Q1 · Space = plain commit |
| Q6 | Should §34 literal candidate stay at index 0? | Its one-script cell now sits beside two-script cells; strip may look uneven. | keep (default) · unchanged but user can already toggle it off |
| Q7 | Setting placement + copy | i18n key + 5 values (hanji / tailo / poj / en / ja) — USER 拍板 文案 per project rule; in-keyboard overlay on iOS/Android too? | `settings.json` new key; naming candidates `hanloTogether` / `showBothScriptsInline` |
| Q8 | Windows | USER listed iOS / Android / macOS only. Windows candidate window is a separate blind-written surface. | state explicitly whether it is in or out — not decided here |

## 9. Best-practices alignment (reference IMEs)

| Mainstream practice | Source `file:line` | Relevance |
|---|---|---|
| Hint / annotation as a **separate field**, renderer decides arrangement | khiin `protos/src/command.proto:118-137` `Candidate { value, key, annotation }` | Matches our `roman` / `hanji` sidechannels; supports "one label" as a renderer choice without touching the wire. |
| Global **output mode** switch Lomaji / Hanji, no third mode | khiin `khiin/src/config/conf.rs:17-20` `OutputMode { Lomaji, Hanji }` | Same two-state model as our swap; khiin has no "both" — 漢羅齊出 would be novel relative to it. |
| Two-line cell, `hanloStatus` flips top/bottom | PhahTaigi `CandidateWordView.swift:107-131`, `HanloStatus.swift` | Exactly our §42 primary/secondary; lomaji-only candidates listed **first** (`:82-96`) = our §34. |
| Candidate `text` + `comment` column; RIME `comment_format` puts reading next to candidate (「注音顯示」) | rime-moetaigi `moetaigi-tsuim.schema.yaml:94-110` (`spelling_hints: 6`, `comment_format`) | RIME frontends render `text comment` on **one line** in horizontal mode — the closest existing precedent for the one-label look. |
| `text` = 漢羅 mixed word, `comment` = `[roman]`; commit derived from the comment in 全羅 mode | rime-phah-taibun `lua/phah_taibun_filter.lua:223-300` | Shows the split "display one thing, commit another" done in a filter layer = our `formatOutputText` seam. |

**Deliberately not adopted**

- rime-phah-taibun's **LKK per-word 漢羅混寫** (`phah_taibun_filter.lua:79-215`, 893 rules) — automatic per-word script selection. Different product; USER asked for both scripts *together*, not rule-based mixing.
- Engine-side combined string (`format!("{} {}", hanji, roman)` in `CandidateMessage`) — §42 already established "primary/secondary, not a formatted string"; the platforms have the fields and the seam. Putting the join in the engine would force every platform to parse it back for width / a11y / Space-alternate.
- Reusing `is_translate_swapped = true` + `output_both_scripts = true` as a stand-in for the mode — the pair already means 括號標註-under-hanji-first (`envelope.proto:70-79`), and three nextword readers would silently change behaviour (§5). If the engine needs to know, a dedicated field is cheaper than overloading two.

## 10. Rough touch list (no estimate of when; sizing only)

| Platform | Files | Nature |
|---|---|---|
| i18n | `i18n/settings.json` (+ `content/` explainer if wanted) | new key, `make i18n` |
| iOS | `SharedSettings` / `EngineSettings` / `SettingsSnapshot?` / `SettingsTab` / `SettingsSelectionOverlay` / `SettingsIcons`; `CandidateCellHelper` (+ `CandidateButtonView`, `ExpandedCandidateGridCell`, width measure); `ActionHandler+Suggestions.formatOutputText`; `ActionHandler+KeyActions` gate; `LayoutConverter` / `TaigiLayouts` or `handleTranslateToggle` (Q3); `KeyboardPreviewPanel` mock; `ThemePreviewEnvironment` | display + settings, ~10 files |
| Android | `PreferenceDataStore` / `PrefHelper` / `EngineSettings`; `InputSettingsScreen` / `SettingsOverlayContent` / `SettingsIcons`; `SmartbarCandidateStrip` + `CandidateOverlayContent` (chooser + `measureCellWidth`); `CandidateClickHandler` ×3 arms; `TextInputKeyHandler` gate; `KeyContent` / layout JSON or `toggleTranslateSwapped` (Q3) | ~9 files |
| macOS | `SettingsStore` / `EngineSettings`; `GeneralSettingsView` (or appearance pane); `CandidateCellContent`; `CandidateMetrics` (a third arrangement or reuse `.inline` with gap); `CandidateDocumentText` (+ `alternateText`, Q5); `AutoSpacePolicy`; `ShortcutActions` handler (Q3); `RetiredSettingsCleanup` note (it currently wipes `outputBothScripts`) | ~8 files |
| engine | only if Q1 ≠ hanji-only: `envelope.proto` `AppConfig` field + `api.rs:165-169` predicate (+ `make build`) | 0–2 files |
| docs | `behavioral-invariants.md` §42 carve-out + new § with `INVARIANT_*`; `continuous-commit-and-display.md:70` if the separator predicate grows a third input | |

Cross-platform parity rule applies (`cross-platform-alignment.md`): define the intended behaviour table (§5 answered) first, then implement per platform.

## 11. Dogfood acceptance draft (to become an **Sn** row once implemented)

- TL `taigi` → strip cell reads `台語 tâi-gí` (single label, order per Q2); no smaller secondary line; index-0 keycap highlight unchanged (S6).
- 文/A key hidden / inert (Q3); `isTranslateSwapped` value irrelevant while ON; turning the mode OFF restores the previous swap state and the key.
- Tap → document receives exactly the Q1 string; auto-space / S10 behave per the roman-ish predicate; POJ mode shows POJ roman (`goá`), `tsiah`/`chiah` identity unchanged (S13).
- 重 tîng / 重 tāng show as two distinguishable cells; `tsia̍h` homophones likewise (#7).
- §34 literal `tâi` still index 0, one-script cell; toggling 顯示原本羅馬字候選 OFF removes it.
- 括號標註 precedence per Q4 — never `彼 hit (彼)`.
- 詞頻 / 詞關聯 rows written with canonical `(hanji, tl)` — verify with the Tab3 viewers after committing in the mode and after switching it off (same rows climb).
- Custom entry with empty hanzi → one-script cell; custom entry equal to a dict word → one cell (custom wins).
- Switch layout to TPS with the mode ON → TPS unchanged (hanji-only cells, S23/S24/S25 unaffected); back to TL → mode resumes.
- macOS: vertical / horizontal / expandable layouts all render one label; Space per Q5; full-width punctuation per Q1.
- Expanded overlays (iOS / Android) pack correctly with wider cells; no truncation of the roman half at default font scale.
