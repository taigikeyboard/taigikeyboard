# 候選詞顯示 (candidate display) modes — research: 漢羅並排 · 漢羅合用 · 羅馬字

> **Type**: Report (dated snapshot) — research only, **nothing implemented**
> **Keywords**: `候選詞顯示`, `漢羅並排`, `漢羅合用`, `羅馬字模式`, `漢羅齊出` (former name), `台語拼音校正`, `hanlo`, `isTranslateSwapped`, `outputBothScripts`, `CandidateCellContent`, `CandidateCellHelper`, `SmartbarCandidateStrip`, `EnglishCandidateStrip`, `§42`, `§34`, `dedupe_by_roman_hanji_span`, `corrector`, `Levenshtein`
> **Date**: 2026-08-30 (Part I) / 2026-09-01 (Part II)
> **Platforms named by USER**: Part I (漢羅合用): iOS, Android, macOS (Windows not named — Q8). Part II (羅馬字): **iOS, Android, macOS, Windows** (USER 2026-09-01; candidate window kept on all platforms per the final decision — §12).
> **Status**: USER 2026-08-30 — 「目前還不打算 implement，先做 research 撰寫文件，供未來參考」; **revived USER 2026-09-01 (「go feature round」)** — Round 1 = picker (漢羅並排 / 羅馬字) implemented on this PR (#662): engine `AppConfig.candidate_display_mode` + two display dedupes (`behavioral-invariants.md` §44, dogfood S26), four platform settings/cell arms. 漢羅合用 = Round 2 (Part I Q1-Q8 still open). No release / version assigned.
> **Amended 2026-09-01 (USER)**: (a) feature renamed **漢羅齊出 → 漢羅合用**; (b) setting reframed from a boolean toggle to a mode picker **候選詞顯示** — **漢羅並排** (today's title/subtitle rendering, default) / **漢羅合用** (Part I's researched mode); (c) later the same day a **third mode 羅馬字** was added (Part II, §12-§21). 羅馬字 went through two design iterations the same day: first proposed as an English-keyboard-style fixed 3-column strip + 台語拼音校正, then **finalized as: today's candidate UI everywhere (all four platforms, candidate window kept), cells roman-only, no 拼音校正** (§12 decision chain). The 3-column + correction research is retained in §13-§16 as reference for a possible future standalone 拼音校正 proposal.

---

# Part I — 漢羅合用 (one-label hanji+roman) mode

## 1. Feature statement (USER, 2026-08-30, verbatim)

> For tl/poj, not tps
> 漢字/羅馬字混合模式，當這個設定開啟後，不需要 swap translate button，不需要分成 title/subtitle，漢字和羅馬字會一併出現。預設設定關閉，設定名稱：漢羅齊出
>
> 可能需要注意的：可能會有重複字；可能影響 Custom dictionary, word association, 詞頻紀錄

> **Update (USER, 2026-09-01, paraphrased)**: the setting is a **mode picker named 候選詞顯示**, not a boolean toggle — mode 1 **漢羅並排** = today's title/subtitle rendering (default); mode 2 **漢羅合用** = the mode researched here, **renamed from 漢羅齊出** (the name in the verbatim statement above); mode 3 **羅馬字** = added later the same day, researched in **Part II (§12-§21)**. Every 漢羅合用 mention below refers to what the original statement called 漢羅齊出.

Read as four requirements plus two risk flags:

| # | Requirement | Reading |
|---|---|---|
| R1 | TL / POJ only, never TPS | TPS stays hanji-only (`effectiveSwapped` forced, §5 display-dedupe by hanji). |
| R2 | 文/A swap key becomes unnecessary | Either hidden or inert while the mode is on. |
| R3 | No title / subtitle split | Cell shows ONE label carrying both scripts, e.g. `台語 tâi-gí`, instead of a primary + smaller secondary. |
| R4 | Both scripts "appear together" | Certain for the **cell**. Whether the **committed document text** also carries both is not stated — see §8 Q1. |
| F1 | 可能會有重複字 | Analysed in §6. |
| F2 | 可能影響 自訂詞 / 詞關聯 / 詞頻 | Analysed in §7. |

Setting surface (USER 2026-09-01): a mode picker named **候選詞顯示** — **漢羅並排** (default; the current title/subtitle rendering, swap key still decides which script leads) / **漢羅合用** (this mode) / **羅馬字** (Part II, §12). Supersedes the original "boolean, default OFF, named 漢羅齊出" framing; "default 漢羅並排" preserves the original default-OFF intent.

## 2. Terminology — three things already called "漢羅" / "both scripts"

The codebase and the reference IMEs use "漢羅" for three different mechanisms. The new mode is a **fourth**; naming it precisely avoids re-implementing one of the others.

| Term | Mechanism | Where | Relation to 漢羅合用 |
|---|---|---|---|
| **§42 cell shows both scripts** (`INVARIANT_CANDIDATE_CELL_SHOWS_BOTH_SCRIPTS`) | Every cell already renders hanji AND roman, as **primary + secondary** visual roles; swap decides which leads. `behavioral-invariants.md:1026-1040` | all three platforms | 漢羅合用 changes the *arrangement* (one label, R3) — the invariant's "never a formatted string" clause (`:1033`) would need an explicit carve-out. |
| **括號標註** (`outputBothScripts`, "Annotate in Brackets") | **Commit-side** rendering: document gets `hit (彼)` / `彼 (hit)`. `i18n/settings.json:49-53`. Cell untouched. | iOS + Android live; macOS setting exists but UI retired, value wiped each launch (`RetiredSettingsCleanup.swift:111`, pinned false per `AutoSpacePolicy.swift:13`). | Sibling. If 漢羅合用 also commits both scripts, the two overlap — §8 Q1/Q4. |
| **§34 literal-roman candidate** (顯示原本羅馬字候選, "漢羅 fast input") | Index-0 roman-only candidate so 漢羅 *mixed-script writers* commit romanization in one tap even in hanji-first mode. `behavioral-invariants.md:843-866`. | engine, TL/POJ | Orthogonal; stays. Under 漢羅合用 its cell reads `tâi` (one script) next to `台 tâi` — see §6 (b). |
| **漢羅混寫** (per-word Han-vs-Lo choice by LKK rules) | Output text mixes hanji words and roman words by rule — `我 beh 去 tshit-thô`. `references/rime-phah-taibun/lua/phah_taibun_filter.lua:79-215`. | reference IME only | **Different feature** (automatic script selection per word). Not what USER asked. Listed in §9 as deliberately not adopted. |

## 3. Current state — per platform (grounded in code)

### 3.1 Swap / 文/A toggle

| | iOS | Android | macOS |
|---|---|---|---|
| Setting key | `SharedSettings.swift:44` `isTranslateSwapped` (default false) | `PreferenceDataStore.kt:46` `keyboard__is_translate_swapped`, `PrefHelper.kt:318` | `SettingsStore.swift:46-49` `"isTranslateSwapped"` (iOS-aligned spelling) |
| Key / UI | `KeyDef.swift:22` `.translate`; bottom row of every Taigi layout `TaigiLayouts.swift:32,46,60,117,131,145,159,201,240`; icon `ButtonImageProvider.swift:37-41`; tap → `ActionHandler+CustomActions.swift:11-21` → `KeyboardContext+Translate.swift:11-25` | `KeyCode.kt:67` `TRANSLATE = -230` in layout JSON (`moe2_halfwidth.json:35-37` …); icon `KeyContent.kt:440`, active bg `:620`; tap → `TextInputKeyHandler.kt:167-169` → `SmartbarManager.kt:521-542`; overlay control panel button `CandidateOverlayContent.kt:220` (`showTranslate = !isTPSLayout`) | **No key, no menu item** — shortcut action only: `ShortcutActions.swift:66,79,85,114` → `TaigiInputController.swift:420-427` (`rerenderCandidatesForDisplayChange`) |
| Side effects of the flag | Full-width char keys `LayoutConverter.swift:43`; confirm-key label 選 vs suán `ButtonTextProvider.swift:194-215` | Full/half-width layout variant `LayoutManager.kt:265-300,323`; `invalidateKeysByCode(TRANSLATE, VIEW_NUMERIC_ADVANCED)` | Full-width punctuation gate `TaigiInputController.swift:749-751`; auto-space gate `:762-772` |
| `effectiveSwapped` | `ComposingManager.swift:181-188` (`isTranslateSwapped \|\| inputMode == .tps`) for engine; commit path recomputes from **layout type** `ActionHandler+Suggestions.swift:81-82,140-141`; auto-space `ActionHandler+KeyActions.swift:150-156` | `CandidateClickHandler.kt:86-89, 225-228, 342-345` (`isTpsLayout \|\| cached`), 4th copy `TextInputKeyHandler.kt:608-613` | No TPS → `settings.isTranslateSwapped` **is** effective |

Every one of those `effectiveSwapped` sites is a place 漢羅合用 must take a position on (§5).

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

### 3.5 Existing `outputBothScripts` wiring = the template for the new setting

| Layer | iOS | Android | macOS |
|---|---|---|---|
| i18n | `i18n/settings.json:49-53` → `make i18n` → `StringKey.swift:186` | → `StringKey.kt:196`, `strings_i18n.xml:189` | → `StringKey.swift:155` |
| Store | `SharedSettings.swift:45,270-273,652` | `PreferenceDataStore.kt:47`, `PrefHelper.kt:320,573-578` | `SettingsStore.swift:50-53`, `EngineSettings.swift:31,81` |
| Contract | `EngineSettings.swift:26-33` (CROSS-PLATFORM INVARIANT comment) | `EngineSettings.kt:54` | `EngineSettings.swift:21-75` |
| Host UI | `SettingsTab.swift:23,60,126-134,344` (`featureSummary("hanloDesign")` info) | `InputSettingsScreen.kt:83,195-203` (`FEATURE_ID_HANLO_DESIGN`) | pane enum `SettingsSplitView.swift:13-42`; `GeneralSettingsView.swift:33-37,79` `@AppStorage` pattern |
| In-keyboard UI | `SettingsSelectionOverlay.swift:18,47,77-79,129`, icon `SettingsIcons.swift:11` | `SettingsOverlayContent.kt:58,97-110`, icon `SettingsIcons.kt:21` | — |
| Engine | `RustEngineBridge+Composing.swift:631-641` `continuousAppConfig` | `RustEngineBridge.kt:1368-1377`, `ComposingManager.kt:923-928` spacing struct | `RustEngineBridge.swift:170-187` |
| Proto | `envelope.proto:70-87` `AppConfig.output_both_scripts = 8` | | |

Note: `featureSummary("hanloDesign")` / `FEATURE_ID_HANLO_DESIGN` — the in-app feature explainer already uses "hanlo" for 括號標註. A new 漢羅合用 entry in `content/` must not collide with that id.

## 4. Engine facts that bound the design

- **Candidate carries both scripts already.** `CandidateMessage` `composing.proto:363-389`: `roman` (display form, POJ-rendered / recased), `optional hanji` (absent = TAILO roman-only), `display_text = hanji ?? roman` (commit / freq key), `canonical_tl` (identity), `mode ∈ {HANT, TAILO, MIXED}` (`lexicon/src/continuous.rs:212,227` — MIXED = roman letters inside the hanji field, e.g. `台BAR`, MOE `VT_MIXED`). **No engine change is needed to render both** — §42 says so (`behavioral-invariants.md:1038`).
- **Swap is not an engine concept** except three readers (§3.4). `lexicon` / `ranking` / candidate construction never look at `is_translate_swapped`.
- **Dedupe key is `(roman, hanji, span)`** — `dedupe_by_roman_hanji_span` `lexicon/src/continuous.rs:2249-2279`; rationale `:889-905` keeps same-hanji-different-roman AND same-roman-different-hanji (Core Principle #7). Post-render pass `composing/src/continuous.rs:294-299` on the rendered roman. TPS-only display dedupe by `(hanji, span)` `:360` — **TPS only, never TL/POJ** (§5 two-tier, `behavioral-invariants.md:132-155`).
- **§34 literal guard**: `composing/src/dispatch.rs:288-296` removes a dict candidate only when `hanji.is_none() && roman == literal.roman`; `台/tâi` is kept beside bare `tâi` on purpose.
- **No engine "hanji + roman in one string" exists** — `combined_display` (`api.rs:334-345`) is nailed-prefix + tail; the bracket form is platform-only (§3.3).
- **Custom dictionary**: SQL `roman NOT NULL, hanzi NOT NULL` (`custom-dictionary.md:39-51`) but empty hanzi allowed → sent as proto-absent `hanji` (iOS `ComposingManager.swift:442-460`, Android `:663-706`, macOS `:309-317`) → `custom_entry_to_candidate` `lexicon/src/continuous.rs:2147-2190` renders TAILO with `display_text = canonical TL`.

## 5. What 漢羅合用 ON has to decide, site by site

Because `effectiveSwapped` is read in ~15 places, the cleanest framing is: **漢羅合用 is a third value of "which script leads", not a fourth boolean beside `isTranslateSwapped`.** Today the state space is `{roman-first, hanji-first} × {plain, 括號標註}`; the new mode adds a display arrangement AND removes the swap axis from the user's reach (R2). USER 2026-09-01 confirmed this shape at the settings surface: 候選詞顯示 is a **picker** (漢羅並排 / 漢羅合用), not a toggle. Note the split: the *setting* is the two-value picker; the *effective* script-lead state stays derived (漢羅並排 × swap = roman-first / hanji-first; 漢羅合用 = the third value). `isTranslateSwapped` keeps its own storage so switching back to 漢羅並排 restores the previous swap state.

| Site | Today (swapped / not) | Under 候選詞顯示 = 漢羅合用 — forced position |
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
| nextword `decide.rs:93` Enter-commits-raw | skipped when swapped | if 漢羅合用 maps to swapped=true, Enter raw-commit is lost → likely want swapped=false semantics here. |
| Full-width keycaps (iOS `LayoutConverter.swift:43`, Android `LayoutManager.kt:265-300`) | follow swap | pick one (probably half-width — roman is being written). |
| 選 vs suán confirm label (iOS `ButtonTextProvider.swift:194-215`) | follow swap | pick one. |
| TPS layout while 漢羅合用 selected (R1) | — | setting ignored; TPS path unchanged (all TPS branches already precede the swap arms). |
| Settings preview (iOS mock candidates) | title/subtitle | must render one-label too; Android preview has no strip. |

## 6. F1 — "可能會有重複字": where duplicates can and cannot come from

| Case | Today | Under 漢羅合用 | Verdict |
|---|---|---|---|
| (a) same roman, different hanji (`tsia̍h` → 食 / 𤆬 / …) | separate rows, differ by subtitle | separate rows `食 tsia̍h` / `𤆬 tsia̍h` — **more** distinguishable | not a duplicate; improved |
| (b) §34 literal `tâi` vs dict `台 tâi` | `tâi` (single line) next to `tâi / 台` | `tâi` next to `台 tâi` | not a duplicate; engine keeps both on purpose (`dispatch.rs:288-296`). Visual proximity is higher because the roman now sits in the same label — cosmetic only. |
| (c) same hanji, different roman (重 tîng / 重 tāng) | two rows both titled 重 in swapped mode | `重 tîng` / `重 tāng` | not a duplicate; **this is the case 漢羅合用 fixes** — today hanji-first mode shows two identical primaries. |
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

1. `is_translate_swapped` also gates *whether* NextWord records / offers (`decide.rs:93`, `filter.rs:245`, macOS `RustEngineBridge+NextWord.swift:117-124` "swap suppresses recording for raw-romanization commits"). Whatever value 漢羅合用 sends there changes learning behaviour for Enter-raw commits — decide deliberately (§5 rows nextword).
2. Roman-only custom entries (empty hanzi) render as one-script cells (`tâi` alone) — same as §34 literal; fine, but the S13 dogfood row should include one.

## 8. Open questions for USER (batched — answer once)

| # | Question | Why it matters | Options seen |
|---|---|---|---|
| Q1 | **What does a tap commit?** | Drives `continuous_word_space`, auto-space, S10, macOS #600/#610, 括號標註 precedence, and whether an engine `AppConfig` field is needed at all. | (a) hanji only (cell is the annotation, document stays clean; = today's swapped) · (b) `漢字 羅馬字` space-joined (the label as typed) · (c) reuse 括號標註 form `漢字 (羅馬字)` · (d) roman only |
| Q2 | Label order in the cell: `漢字 羅馬字` or `羅馬字 漢字`? Fixed or follows the now-hidden `isTranslateSwapped`? | Cell code; §42 carve-out wording; PhahTaigi / khiin both put hanji as the value and roman as hint when in hanji output mode. | fixed hanji-first · fixed roman-first · inherit swap |
| Q3 | 文/A key: **hidden** (layout filter, like TPS) or **inert** (visible, does nothing)? | iOS layouts hardcode the key in 9 rows; hidden = layout-level change + width redistribution; inert = one-line guard. | hidden · inert |
| Q4 | Precedence with 括號標註 when both ON | Avoid `彼 hit (彼)`. | 漢羅合用 disables 括號標註 UI · 括號標註 wins · mutually exclusive picker |
| Q5 | macOS Space (= commit alternate script, #610) under 漢羅合用 | No annotation exists to read; today `.ignored` for one-script cells. | Space inert · Space commits "the other" per Q1 · Space = plain commit |
| Q6 | Should §34 literal candidate stay at index 0? | Its one-script cell now sits beside two-script cells; strip may look uneven. | keep (default) · unchanged but user can already toggle it off |
| Q7 | Setting placement + copy — **partially decided 2026-09-01**: picker 候選詞顯示, values 漢羅並排 (default) / 漢羅合用 / 羅馬字 (Part II) | Still open: i18n key names + the other 4 locale values (tailo / poj / en / ja) — USER 拍板 文案 per project rule; in-keyboard overlay on iOS/Android too? | `settings.json` new key; naming candidates for the picker `candidateDisplayMode` with values e.g. `hanloSideBySide` / `hanloCombined` / `romanOnly` |
| Q8 | Windows | USER listed iOS / Android / macOS only. Windows candidate window is a separate blind-written surface. | state explicitly whether it is in or out — not decided here |

## 9. Best-practices alignment (reference IMEs)

| Mainstream practice | Source `file:line` | Relevance |
|---|---|---|
| Hint / annotation as a **separate field**, renderer decides arrangement | khiin `protos/src/command.proto:118-137` `Candidate { value, key, annotation }` | Matches our `roman` / `hanji` sidechannels; supports "one label" as a renderer choice without touching the wire. |
| Global **output mode** switch Lomaji / Hanji, no third mode | khiin `khiin/src/config/conf.rs:17-20` `OutputMode { Lomaji, Hanji }` | Same two-state model as our swap; khiin has no "both" — 漢羅合用 would be novel relative to it. |
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
- 文/A key hidden / inert (Q3); `isTranslateSwapped` value irrelevant while 漢羅合用 selected; switching back to 漢羅並排 restores the previous swap state and the key.
- Tap → document receives exactly the Q1 string; auto-space / S10 behave per the roman-ish predicate; POJ mode shows POJ roman (`goá`), `tsiah`/`chiah` identity unchanged (S13).
- 重 tîng / 重 tāng show as two distinguishable cells; `tsia̍h` homophones likewise (#7).
- §34 literal `tâi` still index 0, one-script cell; toggling 顯示原本羅馬字候選 OFF removes it.
- 括號標註 precedence per Q4 — never `彼 hit (彼)`.
- 詞頻 / 詞關聯 rows written with canonical `(hanji, tl)` — verify with the Tab3 viewers after committing in 漢羅合用 and after switching back to 漢羅並排 (same rows climb).
- Custom entry with empty hanzi → one-script cell; custom entry equal to a dict word → one cell (custom wins).
- Switch layout to TPS with 漢羅合用 selected → TPS unchanged (hanji-only cells, S23/S24/S25 unaffected); back to TL → mode resumes.
- macOS: vertical / horizontal / expandable layouts all render one label; Space per Q5; full-width punctuation per Q1.
- Expanded overlays (iOS / Android) pack correctly with wider cells; no truncation of the roman half at default font scale.

---

# Part II — 羅馬字 (roman-only cells) mode

> Added 2026-09-01. Research only, nothing implemented. **Final shape (§12 decision chain): today's candidate UI on all four platforms — candidate window/strip/overlay kept — with cells rendering roman only; the 3-column layout and 台語拼音校正 were considered and dropped the same day.** §13-§16 retain the dropped-direction research (English 3-column survey, engine tolerance state, correction best practices) as reference for a future standalone 拼音校正 proposal; §17 onward describe the final shape. Grounded in: three code surveys (English strip UI / engine tolerance state / reference-IME correction practices, 2026-09-01) + direct source reads of the freshly re-cloned `references/librime` and `references/khiin-rs` (the earlier survey ran against an empty `references/`; librime/khiin claims below are now source-verified).

## 12. Feature statement (USER, 2026-09-01, verbatim)

> 我想再加一個模式叫做羅馬字，在這個模式，候選詞欄位會像英文鍵盤那樣三欄顯示，並且有台語拼音校正的功能

Supplements (USER, 2026-09-01, verbatim):

> 「羅馬字」模式，仍然要有工具列，只是原本的 overlay 候選詞選單，改為類似英文鍵盤的三欄式

> 這個功能 is for ios,android,macos,windows, 在 desktop 的話，「羅馬字」模式不會有候選窗，我記得在英文還是越南語的 desktop 不會有候選窗

**Final decision (USER, 2026-09-01, verbatim, supersedes the 3-column / correction / no-desktop-window statements above):**

> 你覺得羅馬字要有候選窗嗎？包含 mobile,就是取消原本的三欄式和拼音校正，而是採用目前的方式，只是 title 只有羅馬字

> ok,保留候選窗,不需要做拼音校正

Context for the final decision: the assistant recommended keeping the candidate UI because (a) tone-optional input (`taigi` → `tâi-gí`, `si` → tone family) is the mode's killer affordance and a fixed 3-slot strip structurally cannot hold a tone-family fan-out — the RIME Taigi schemas (librime / rime-phah-taibun, both desktop) all do 全羅 via candidate windows; (b) a roman-only cell arm reuses every existing surface on all four platforms (Core Principle #6 consistency) where 3-column required three platform-specific novelties; (c) desktop correction without a window has no substitution-style precedent (§15 + the UniKey/EVKey validity-gate-only finding); (d) 拼音校正 is display-mode-independent and better sequenced as a future standalone proposal (§13-§16 kept for it).

Final requirements:

| # | Requirement (final) | Reading |
|---|---|---|
| R5′ | Candidate UI = **today's, everywhere** — mobile strip + expand overlay, macOS panels, Windows candidate window all stay | Only the **cell rendering** changes. Supersedes R5 (3-column) / R8 (overlay removed) / R10 (no desktop window). |
| R6′ | **No 拼音校正** | Dropped from this mode. §13-§16 retained as the research base for a possible future standalone, mode-independent 拼音校正 proposal. Supersedes R6. |
| R7 | Toolbar unchanged | Now trivially satisfied (nothing around the strip changes at all). |
| R9 | All four platforms: iOS, Android, macOS, Windows | Desktop keeps its candidate window (R5′). |
| R11 | Cell = **single roman title, no subtitle** | 「title 只有羅馬字」. No hanji anywhere in candidate UI. Same-roman rows need a display dedupe (§17/§18). |

Implied but not stated (confirm in §19): tap commits roman; TL/POJ only, never TPS/English (mirrors Part I R1).

## 13. [DROPPED direction — reference only] English 3-column strip, per platform (grounded in code)

> The 3-column layout was dropped by the §12 final decision. Survey kept because it maps the English-mode strip architecture (useful for any future English-path work) and grounded the drop rationale.

| | iOS | Android | macOS |
|---|---|---|---|
| 3-column layout | **None app-side.** English suggestions render via KeyboardKit's closed-source `Autocomplete.Toolbar`, injected at `TaigiKeyboardView.swift:349` and branched at `CandidateSuggestionsRow.swift:29` (English gets no separator, no expand chevron). App never enumerates slots or sets widths. | **Real fixed 3-column**: `SmartbarCandidateStrip.kt:112-163` `EnglishCandidateStrip` — `for (slot in 0..2)` (`:143`), `Modifier.weight(1f)` equal widths (`:146`), 1dp×24dp `EnglishDivider` between slots (`:158-160`, `:350-358`), empty slot = invisible spacer (`:314-317`), **no LazyRow / no scroll** (`:107-110`). | **No English mode at all** (removed 2026-08-26, `TaigiInputController.swift:132-138`; `EngineSettings.swift:9-12` has only tl/poj). Candidate panels are Taigi-only. |
| Cell | KK `ToolbarItemStyle`; app styles only `style.item` colors (`CandidateSuggestionsRow.swift:56-61`), never `autocorrectItem` — KK's autocorrect highlight treatment exists but is unreached | single centered `BasicText`, pressed-highlight only, forced `FontFamily.SansSerif` (`:130`, `:301-347`); **no index-0 keycap fill** (that exists only in the Taigi cell `:193-198`) | — |
| Strip switch | view-level branch `CandidateSuggestionsRow.swift:27-42`; service swap `KeyboardViewController+Setup.swift:184-199` | two ComposeViews (`SmartbarView.kt:114-143`), container visibility `ToolbarManager.kt:299-315`, `SmartbarContainer.ENGLISH_CANDIDATES`; data-side `SmartbarManager.kt:624-643` `updateEnglishCandidates` (`take(3)` `:632`) + `CandidateMode.English` | — |
| Suggestion cap | service returns ≤3 (`EnglishAutocompleteService.swift:78,99`) | `MAX_SUGGESTIONS = 3` (`EnglishAutocompleteService.kt:33`) | — |
| English correction today | Apple `UITextChecker` completions + misspelling **guesses** (`EnglishAutocompleteService.swift:14,73-103`); suggestions all `type: .regular` — `isAutocorrect` never set; **no auto-replace on space** (`ActionHandler+KeyActions.swift:163-168` inserts literal space) | bundled 30k `english_freq.txt` + `EnglishWordMatcher.kt:46-65`: prefix binary search + **OSA edit distance ≤2** (`MAX_EDIT_DISTANCE=2`, `MIN_CORRECTION_LENGTH=3` `:136-138`); tap-to-commit only, **no auto-replace on space** (`TextInputKeyHandler.kt:410-418`) | — |
| Toolbar (R7 reference) | English container keeps the toolbar row; only the candidate strip content differs | `smartbar.xml:39-103` — `english_candidates_container` sits beside, not instead of, the tool row | — |

Structural takeaways: (a) Android already owns the exact UI the USER described, including "toolbar stays, candidates become 3 fixed slots, no overlay"; the 羅馬字 mode is a new `CandidateMode` (or a parameterized reuse) fed by Taigi engine data instead of `english_freq.txt`. (b) iOS has **no reusable 3-slot component** — English mode delegates to KeyboardKit; a Taigi 羅馬字 strip means a new SwiftUI 3-slot row (KK's toolbar renders KK `Autocomplete.Suggestion`s from the KK autocomplete context — squeezing engine candidates through it inverts the data flow; building the row is simpler and keeps §42-family styling in our hands). (c) macOS has neither an English mode nor a smartbar, and Windows has its own candidate window (`windows/crates/taigi-windows-tsf/src/ui/candidate_window.rs`, `candidate_list_element.rs`; core `taigi-windows-core/src/candidates/`) — R10 resolves both: desktop suppresses the candidate window entirely in this mode, so neither platform needs a 3-column analogue. (d) In-house precedent for the *correction* half already ships in the English path: Android OSA≤2 matcher + iOS UITextChecker guesses — both **suggestion-only, never auto-replace** — a deliberate house convention the Taigi corrector should follow unless USER says otherwise (Q13).

## 14. [FUTURE-REFERENCE — 拼音校正 dropped from this mode] Spelling tolerance in the engine: none (grounded in code)

> 拼音校正 was dropped by the §12 final decision. §14-§16 are the research base for a future **standalone, display-mode-independent** 拼音校正 proposal — do not delete.

- **Zero tolerance today.** No edit-distance / fuzzy / adjacency code or crate anywhere in `engine/` (`Cargo.lock` has no `strsim`/`levenshtein`/`fuzzy-matcher`); `tps_ambiguity.rs:26` explicitly scopes cross-phoneme fuzzy matching OUT of the §35 work.
- **Invalid input = verbatim, silently zero candidates** (§10.2 / invariant I1): `syllabifier/tl.rs:84-119` BFS gated by `SyllableInventory::contains_in` — no hit → empty endings, no panic; partial-prefix fallback `continuous.rs:1152-1177` then misses the FST (`shadow.rs:85-90`: "never a wrong-tone hit"); the user still sees the §34 literal (`dispatch.rs:288-297`). **A corrector must not break this**: correction adds *candidates*, the preedit stays verbatim (contrast: our TPS auto-correct `knowledge/tps-auto-correct-rules.md` mutates `rawInput` at keystroke time — a different, TPS-only insertion point that must not be copied for TL/POJ).
- **The FST layer has the hooks half-built**:
  - `fst = "0.4"` (resolved 0.4.7) is declared **without** the `levenshtein` feature (`engine/Cargo.toml:44`, lock has no deps block). Enabling it gives `fst::automaton::Levenshtein` usable with the already-used `Set::search` streaming API, pulling exactly one new crate (`utf8-ranges`, Unlicense OR MIT — `deny.toml` allows MIT but not Unlicense; verify cargo-deny resolves the OR before relying on it).
  - In-tree precedent for a custom non-exact automaton: `lexicon/src/tps_pattern.rs:39` `impl fst::Automaton for TpsKeyPattern`, already consumed by `syllable_inventory.rs:137` and `prefix_index.rs:196`, already carries a `substitution_count` (`:31`) and already solves the "don't edit inside the `tl:` family prefix / `0xFF‖rowid` suffix" problem (`:82-88`) that a naive `Levenshtein` over `dictionary.fst` would trip on.
  - `syllables.fst` is tiny (17 KB vs dictionary.fst 15 MB) and holds exactly the right unit — every valid syllable, numeric + toneless keys, three families (`syllable_inventory.rs:1-22`) — making **syllable-level** tolerance cheap; word-level tolerance over dictionary.fst is the expensive path.
- **Ranking has a ready-made slot**: `coverage_kind` is the leading `SortKey` dimension (`lexicon/src/continuous.rs:120,130`, sort at `~:2302`) — the existing "this class ranks strictly below full hits regardless of score" mechanism a `corrected` class would parallel.
- **Wire headroom**: `CandidateMessage` has no corrected flag; next free tag = 11 (`composing.proto:363-389`). `AppConfig` next free tag = 9 (`envelope.proto:78-87`); `FetchAtPos` next free tag = 7 with two sentinel-style toggles as precedent.
- **English mode is unaffected**: engine treats `"english"` as TL-with-exclusions for phonetics/case calls only; both platforms' English suggestion pipelines are fully platform-side (§13).

## 15. [FUTURE-REFERENCE] Best-practices alignment — correction (reference IMEs, source-verified 2026-09-01)

`docs/references/mainstream-ime-comparison.md` has **no correction/fuzzy topic row** (nine topics, none cover it) — a documented gap; add one when this ships. Per-IME findings, now from real source:

| Practice | Source `file:line` | Relevance |
|---|---|---|
| **librime corrector — the only real prior art.** `speller/enable_correction` gates a `Corrector` with two engines combined: `NearSearchCorrector` — BFS over the prism trie substituting **QWERTY-adjacent** chars (hardcoded `keyboard_map`), +1 distance per substitution, substitution-only; `EditDistanceCorrector` — SymSpell-style prebuilt prism (`SymDeleteCollector`) + Levenshtein / threshold-bounded `RestrictedDistance`. | `references/librime/src/rime/dict/corrector.{h,cc}` (`keyboard_map` `corrector.cc:19-45`; `NearSearchCorrector::ToleranceSearch` `:246-288`; `Combine(New<NearSearchCorrector>(), ed_corrector)` `:315`) | Two composable engines; adjacency model is layout-specific (ours would be our QWERTY/TPS layouts, not RIME's). |
| **Correction is a lattice-edge property, not an input rewrite.** Syllabifier runs exact `CommonPrefixSearch`, then `ToleranceSearch(…, tolerance=5)`; any match not in the exact set gets `props.is_correction = true; props.credibility = kCorrectionCredibility` = **−4.605 = log(0.01)**; correction edges never establish vertex type. | `references/librime/src/rime/algo/syllabifier.cc:29,70-135,177-179` | The exact shape our engine wants: preedit verbatim (§10.2 kept), corrected *candidates* penalized into the ranking. Confirms the −4.605 figure `mainstream-ime-comparison.md:212` had second-hand. |
| **Corrections are capped and de-prioritized, not hidden**: `max_corrections_ = 4` per translation; iteration skips corrections beyond the cap; a user-dict entry that is itself a correction **loses** its usual user-dict priority and falls back to weight comparison (comment: 系統原文 > 用戶糾錯). | `references/librime/src/rime/gear/script_translator.cc:172,541-552,614-626` | Calibration numbers + the subtle rule that correction must not ride user-dict boost — maps to our custom-dict rank-0 policy. |
| **khiin-rs has NO correction** — grep for fuzzy/typo/distance over `khiin/src/` is empty; `SectionType ∈ {Plaintext, Hyphens, Punct, Splittable}`, unknown input passes through as `Plaintext` verbatim. The "Exact → Fuzzy → Segmented" tier in `docs/references/khiin-reference.md:128` does not exist in source. | `references/khiin-rs/khiin/src/input/parser.rs:6-22` | Kills the doc's only khiin "fuzzy" lead; khiin shares our verbatim philosophy. Fix `khiin-reference.md:128` when convenient. |
| **rime-phah-taibun conflates orthography variants with laxness** in one zero-penalty `derive` list (`ts/ch`, `ing/eng`, `ua/oa` = POJ↔TL orthography; `ph/f`, `nng/ng`, drop `-h` = actual tolerance) — all rank **tied with exact** because `derive` carries no penalty. | `references/rime-phah-taibun/schema/phah_taibun.schema.yaml:77-105`; penalty table `docs/references/rime-reference.md:133-160` | The anti-pattern to avoid: our POJ/TL orthography unification is already a zero-cost *family/canonicalization* concern (Core Principle #3/#7); 校正 must be a separate, penalized layer. Confusion sets come from `knowledge/taigi-phonetics-reference.md` + `taigi-converter/`, never inferred. |
| **Our own English matcher is the in-house calibration**: OSA ≤2, min word length 3, prefix-match first, corrections ranked after prefix hits, suggestion-only. | `android/.../EnglishWordMatcher.kt:46-65,118-145` | House precedent for distance bound + UX (never auto-replace). |

**Deliberately not adopted**: librime's QWERTY `keyboard_map` verbatim (our layouts differ; TPS rows are not QWERTY); rime-phah-taibun's zero-penalty derive-as-tolerance; TPS auto-correct's keystroke-time input mutation as a TL/POJ mechanism; word-level Levenshtein over the 15 MB `dictionary.fst` as the first cut (syllable-level over 17 KB `syllables.fst` first).

## 16. [FUTURE-REFERENCE] 台語拼音校正 — proposed shape (deferred; not part of 羅馬字 mode)

1. **Insertion point**: engine, span-local, syllable-level. After exact `valid_span_endings` produces nothing (or optionally alongside), run a tolerance search of the current span against `syllables.fst` (`Levenshtein` automaton with distance 1, and/or a `TaigiKeyPattern` adjacency automaton modeled on `TpsKeyPattern`), then feed the corrected syllable keys through the normal lexicon fetch. Preedit stays verbatim (§10.2); correction candidates appear in the strip only. Platform-side correction (English-style wordlist matcher) is rejected: the Taigi dictionary lives in the engine, and three platforms would each re-implement it — violates cross-platform-alignment economy.
2. **Ranking**: new `RawCandidate` axis parallel to `coverage_kind` (corrected class strictly below exact/full hits), plus a librime-style constant penalty inside the class; cap corrected candidates per fetch (librime: 4). A corrected candidate is an ordinary dict row reached via a corrected key — identity stays `(hanzi, canonical_tl)` (#7), so 詞頻/詞關聯/自訂詞 keys are untouched by construction; whether custom-dict entries participate in correction and whether a corrected commit should learn frequency at full weight = Q13/Q15.
3. **Wire**: `CandidateMessage` field 11 `bool is_correction` (display marking + platform-side UX decisions). Correction is display-mode-independent (§12 final decision), so it does **not** by itself require the engine to know the 候選詞顯示 mode; if it ever needs a config switch that is its own `AppConfig` field, chosen when the proposal is written (Part I §5/§10's "dedicated field beats overloading `is_translate_swapped`/`output_both_scripts`" still applies).
4. **Confusion sets / distance model**: start with distance-1 Levenshtein over syllable bodies + tone-digit-slip tolerance as separate cheap classes; any phoneme-confusion pairs (n/l, in/ing …) must be sourced from `knowledge/taigi-phonetics-reference.md` + `taigi-converter/` (Core Principle #3), never from intuition — and kept distinct from POJ/TL orthography mapping, which is already handled by families/canonicalization at zero cost (§15 anti-pattern).
5. **License note**: `fst/levenshtein` pulls `utf8-ranges` (Unlicense OR MIT); `deny.toml` currently lists MIT but not Unlicense — verify cargo-deny accepts the OR-resolution before enabling the feature.

## 17. Sites the 羅馬字 mode forces a position on (final shape)

| Site | Today | Under 候選詞顯示 = 羅馬字 |
|---|---|---|
| Cell content | title/subtitle (並排) or one label (合用) | **single roman title, no subtitle** (R11) — the engine `roman` field (display form: POJ-rendered / recased), not `canonical_tl`. MIXED rows (`台BAR`) show their roman (`tâi-bà`). **This is a USER-approved exception to §42's core clause, not a third arrangement**: §42 (`behavioral-invariants.md:1026-1040`) pins that one script alone cannot identify a candidate (重 tîng/tāng, tsia̍h 食/𤆬) and that no setting turns the second script off — 羅馬字 deliberately accepts that information loss (全羅 writing does not distinguish homophones either) and pays for it with the display dedupe in the "Same-roman rows" row. The §42 rewrite must say so explicitly (USER 2026-09-01 「title 只有羅馬字」), and §5's "entries without hanzi are always kept" clause (`:137`) needs a mode-specific carve-out for the §34 literal row below. |
| Strip / overlay / panels / window | scroll strip + expand chevron (mobile); macOS 3 layouts; Windows candidate window | **all unchanged** (R5′) — scroll, expand, S6 index-0 keycap highlight, paging, slot keys all stay. Only the cell renders differently. |
| Same-roman rows | distinct by hanji (subtitle/title) | visually identical roman-only cells → **display dedupe by `(rendered roman, span)`**, mirroring the TPS `(hanji, span)` display dedupe (§5 two-tier `behavioral-invariants.md:132-155`). Engine-side (one implementation for 4 platforms; nextword needs its own, see the nextword row). **Insertion point ≠ the TPS one**: `dedupe_display_hanji_for_tps` runs inside `assemble_candidates` (`composing/src/continuous.rs:1569-1575`), but the §34 literal is prepended **later** in `dispatch.rs:288-296` — a pass placed beside the TPS one never sees the literal (next row). Place it in `dispatch::handle_fetch_at_pos` after the literal prepend, or lift the TPS pass to the same spot so both display-tier passes share one seam (Codex 2026-09-01 review: one complete post-prepend display-tier pass preferred). Mode signal carrier → Q19. Survivor = first surviving row in post-prepend order — the top-ranked row for sorted candidates, but the §34 literal whenever it is in the group (it is inserted at index 0 *after* sort, it is not ranked) (Q9′); **custom rank-0 does not guarantee survivorship** — `source_rank` sits behind coverage / tier / recency / score / freq / bytes in `SortKey` (`lexicon/src/continuous.rs:2362-2380`); it only wins identical-`(roman, hanji, span)` collisions (`:2237-2249`) — see §18. |
| §34 literal | index 0; guard drops identical **bare-roman** dict rows only (`dispatch.rs:288-296`) | stays index 0 (inserted first → first-seen). Today's guard keeps `台/tâi` beside bare `tâi` because the two **commit** differently (Codex pre-impl F5, comment at `:291-294`); under 羅馬字 they render and commit identically (`tâi`), so the guard's reason vanishes and the two cells would sit side by side unless the row above runs after the prepend (or the guard is widened to hanji-bearing rows while the mode is 羅馬字). Learning consequence: the literal's `display_text` is roman → `(tâi, tâi)` roman bucket (Q9′ option (b) by construction), whereas the collapsed dict row would have learned `(台, tâi)` — the most common tap in this mode learns the roman bucket, not a hanji pair. Decide in Q9′ whether that is acceptable or whether the literal should yield to the dict row's identity when a same-roman dict row exists. |
| 文/A swap key | visible (mobile); **desktop = shortcut-only, no key/menu on both** — macOS `ShortcutActions.swift:66-114`; Windows surveyed 2026-09-01: `ShortcutAction::ToggleTranslateSwapped`, default chord bare `` ` `` (macOS-aligned by design, `shortcut_actions.rs:82-90` "the Mac's own default"; key-sink-consumed only while TIP active), rebindable in the settings shortcuts pane (`winui/pages/shortcuts.rs:38` renders a recorder row per `ShortcutAction::ALL`; `winui/window.rs:851-864` is the recorder's test), handler flips `isTranslateSwapped` + re-renders the list in place with a 「漢字/羅馬字代先」 flash (`session.rs:654-670`) | no hanji anywhere → mobile: hidden or inert (Part I Q3 option set); desktop both platforms: nothing to hide → shortcut becomes no-op; whether the flash shows an "inactive in this mode" hint or stays silent is the only UX residue → Q11. |
| Commit string | 3-arm formatter (§3.3) | roman unconditionally (= today's not-swapped arm; same commit + `display_text`/`canonical_tl` sidechannels, so learning semantics are literally today's roman-first semantics). 括號標註 meaningless → disable its UI while 羅馬字 selected (recommended; Part I Q4 family). |
| `continuous_word_space` / auto-space / S10 | predicate `!swapped ‖ both` | roman-ish = **true** unconditionally — falls out of sending `is_translate_swapped=false` + `output_both_scripts=false` while 羅馬字 is selected (`api.rs:164-170` already returns true for that pair), so **no engine reader changes**. Each platform computes the effective pair in its `AppConfig` snapshot builder (one site per platform — the same place TPS already forces `effectiveSwapped`), and its own auto-space / S10 / #600 gates read the same derived pair. The engine needs a mode signal only for the display dedupe (Q19). |
| nextword | `shape_prediction` text/subtitle; swapped gates recording | send swapped=false semantics (Enter-raw recording stays, `decide.rs:93`; `filter.rs:245` roman-empty drop stays). **Prediction duplicates are a second, separate collapse site**: nextword merges by `(hanzi, tl)` (`filter.rs:53-60`) and `shape_prediction` (`:238-260`) emits the same `text` with a different `subtitle` for 同音異字 — predictions never pass through the continuous dedupe, so a roman-only prediction strip shows duplicates unless nextword gets its own mode-aware `(text, span-less)` display dedupe after its sort (engine, `filter.rs`) → Q20. |
| macOS / Windows Space = alternate script (#610 + its Windows port) | reads the cell's annotation | roman-only cells have no annotation → alternate text nil → Space `.ignored` / `Ignored` before reaching the engine. macOS `CandidateDocumentText.swift:70-79`, `ComposingManager.swift:371-374`; Windows isomorphic: `taigi-windows-core/src/composing/document_text.rs:86-91` → `composing/manager.rs:317-319`, and the `auto_space.rs:44-45` alternate predicate is never reached. **Zero work on both platforms; regression-pin both in dogfood (§21).** |
| macOS full-width punctuation (#600) | hanji output only | roman output → half-width; follows the same mode predicate. |
| TPS / English | — | TPS: setting ignored (as Part I R1) — Q12 to confirm; English mode path untouched. |
| Settings preview (`KeyboardPreviewPanel`) | mock title/subtitle candidates | third arrangement: roman-only mocks. |
| Custom dict roman-only entries | already render one-script | unchanged — they are the existing precedent for a roman-only cell. |

## 18. Duplicates / store impact (Part I §6/§7 rerun for 羅馬字)

- Same-roman different-hanji rows (`tsia̍h` 食/𤆬) become **visually identical roman-only cells** — the one new duplicate class this mode creates. Engine identity dedupe `(roman, hanji, span)` correctly keeps both (#7 — different words); the fix is a **display-tier** `(rendered roman, span)` dedupe like TPS's `(hanji, span)` one, but placed after the §34 prepend in `dispatch` (§17 "Same-roman rows"). **Two collapse sites, not one**: continuous candidates (dispatch, post-literal) and nextword predictions (`filter.rs`, §17 nextword row) — both engine-side. Same-roman different-**tone** rows (tîng vs tāng) render differently and correctly stay separate.
- Stores: no schema/key change. Commit in this mode = today's roman-first commit path verbatim (same `display_text` / `canonical_tl` / `association_tl` sidechannels), so 詞頻/詞關聯 behave exactly as roman-first mode does today. The only nuance: **who learns when a collapsed group is tapped** — the tap has no hanji intent, and the learned pair's boost is cross-mode shared. Three options + a hybrid analysed in Q9′ (survivor-pair recommended). 漢羅合用 has no such nuance: one cell = exactly one pair — tapping is *more* precise than today's hanji-first mode (§6(c)).
- 自訂詞: identity and storage untouched. **But visibility is not guaranteed**: custom rank-0 wins only the identical-`(roman, hanji, span)` collision (`lexicon/src/continuous.rs:2237-2249`), not the same-roman display group — `source_rank` is the second-to-last `SortKey` dimension (`:2362-2380`), so a custom `tsia̍h/X` with lower freq/score than dict 食 collapses **behind** it and disappears from the strip in this mode. If a user's own entry must stay reachable, the display dedupe needs a custom-first survivor rule inside the group (folded into Q9′). Roman-only custom entries (empty hanzi) are already one-script cells and collapse with any same-roman dict row exactly like the §34 literal does.

## 19. Open questions for USER (Part II batch — pruned to the final shape)

Resolved 2026-09-01 by the §12 decision chain: cells roman-only + commit roman (old Q9); slot policy (old Q10 — obsolete, no 3-column); 校正 UX + learning (old Q13/Q15 — dropped with 校正; collapse-survivor residue moved to Q9′); platform scope + candidate window kept everywhere (old Q14); nextword in 3 slots (old Q16 — obsolete, strip unchanged). Stale Q10/Q13/Q14/Q15 mentions inside the [FUTURE-REFERENCE] sections §13-§16 refer to this superseded numbering. Remaining:

| # | Question | Why it matters | Options seen |
|---|---|---|---|
| Q9′ | **Learning attribution for a collapsed same-roman group** (`tsia̍h` 食/𤆬 → one cell, one tap, several `(hanji, tl)` pairs under it). Scope note: one-hanji-many-readings (重 tîng/tāng) is NOT affected — different roman → separate cells, each learns its own pair (S15 intact); only 同音異字 collapse. | The tap carries no hanji intent — 全羅 writing itself does not distinguish homophones — so any attribution is a heuristic; but the chosen pair's boost is **cross-mode shared**, so it pre-orders the homophones the user will later see in 並排/合用 without ever having chosen (rich-get-richer). 詞關聯 recall is safe under every option (keyed on canonical tl, S11 both directions). Two sub-cases the survivor rule must also settle (§17/§18): (i) the §34 literal collapses with a same-roman dict row and, being inserted first, is the survivor — its `display_text` is roman, so the most frequent tap learns the roman bucket regardless of option; (ii) a custom entry in the group is not the survivor by default (`source_rank` is nearly last in `SortKey`) — custom-first inside the group, or accept that it hides. | (a) **survivor-pair** — learn the surviving row's `(hanji, tl)`: the top-ranked sorted row, or — when the §34 literal is in the group — the literal's roman bucket, since it is inserted at index 0 after sort (sub-case (i)) (recommended: consistent with what the strip shows, zero new write rules) · (b) roman-bucket — learn `(roman, tl)` like today's hanji-absent candidates (§34 literal / roman-only custom already write `display_text = roman`), but splits learning across modes, against S11/S13 · (c) boost the whole group — inflates 詞頻 and multi-writes 詞關聯, rejected · hybrid mitigation if (a)'s cross-mode pre-ordering is unacceptable: collapsed-group taps record 詞關聯 only, no hanji-pair 詞頻 — first mode-dependent write rule in the stores, not recommended as v1 |
| Q11 | 文/A key while 合用/羅馬字 selected — mobile: hidden or inert (Part I Q3 rerun; iOS layouts hardcode the key in 9 rows)? Desktop (macOS + Windows, both shortcut-only, both default bare `` ` `` — §17 survey): shortcut no-op; flash a "no effect in this mode" hint or stay silent? | same trade-offs as Part I Q3; desktop surveyed 2026-09-01, macOS/Windows deliberately isomorphic | mobile hidden · mobile inert; desktop silent no-op · desktop hint flash |
| Q12 | Mode scope: TL/POJ only (TPS/English excluded)? | mirrors Part I R1 | confirm |
| Q17 | 括號標註 while 羅馬字 selected: disable its UI (recommended — commit is roman-only, the bracket form is unreachable) or leave visible-but-inert? | settings coherence; Part I Q4 family | disable UI (recommended) · inert |
| Q18 | Part I Q8 tension: 羅馬字 on Windows puts the 候選詞顯示 picker on Windows — does 漢羅合用 also ship there, or is the Windows picker 並排/羅馬字 only? | Windows settings surface | — |
| Q19 | **Mode-signal carrier for the engine display dedupe.** The engine needs the mode only for the two display dedupes (§17); spacing / nextword readers are satisfied by the platform sending the derived `swapped=false, both=false` pair. | `AppConfig` enum = one explicit contract across every request (and reusable if 漢羅合用 ever needs the engine); a `FetchAtPos` sentinel is narrower and matches the field's actual responsibility (precedent: field 6 `literal_roman_candidate_disabled`, `composing.proto:205`) but does not reach nextword. Codex 2026-09-01: AppConfig enum defensible as a mode contract, "engine spacing needs it" is not a valid reason. | `AppConfig` field 9 enum (`candidate_display_mode`) · `FetchAtPos` field 7 + a nextword request flag · decide at Codex pre-impl |
| Q20 | **NextWord prediction collapse** — engine-side `(text)` display dedupe in `nextword/src/filter.rs` after sort (mode-aware, mirrors the continuous one), or a platform presentation policy? | Predictions bypass the continuous pipeline; without it the roman-only prediction strip repeats `tsia̍h` once per learned homophone. Engine-side keeps 4 platforms + iOS/Android/macOS/Windows prediction renderers identical. | engine `filter.rs` (recommended) · platform-side |

## 20. Rough touch list (sizing only — final shape)

Dramatically smaller than the dropped 3-column + 校正 direction (no new strip components, no tolerance engine, no CandidateMessage field):

| Layer | Files | Nature |
|---|---|---|
| engine | mode-signal carrier per Q19 (`envelope.proto` AppConfig field 9 enum **or** `FetchAtPos` field 7 sentinel) + `derived.rs` mirror + proto regen; `(rendered roman, span)` display dedupe in `dispatch::handle_fetch_at_pos` **after** the §34 prepend (`dispatch.rs:288-296` area — not beside the TPS pass in `assemble_candidates`, §17 "Same-roman rows"); survivor rule per Q9′ (custom-first inside the group if chosen); nextword prediction dedupe in `nextword/src/filter.rs` post-sort (Q20); tests incl. literal-vs-`台/tâi` collapse, custom-in-group, prediction collapse | ~5-6 files; `make build` |
| iOS | `CandidateCellHelper` roman-only arm (title = roman, subtitle = nil — `CandidateButtonView`/grid cell need no change if subtitle is nil-safe); `AppConfig` snapshot builder sends derived `swapped=false, both=false` under 羅馬字 (`RustEngineBridge+Composing.swift:631-641`); settings picker (3rd value); preview mocks | ~5 files |
| Android | strip + overlay chooser roman-only arm (`SmartbarCandidateStrip.kt:198-232`, `CandidateOverlayContent.kt:358-364`); `AppConfig` snapshot builder derived pair (`RustEngineBridge.kt:1368-1377`); settings picker | ~5 files |
| macOS | `CandidateCellContent` roman-only arm (annotation = nil; #610 Space then auto-ignores); `RustEngineBridge.swift:170-187` derived pair; settings picker; `AutoSpacePolicy` mode predicate | ~4-5 files |
| Windows | `document_text.rs` `CandidateCellContent::cell` roman-only arm (annotation = `None`; Space then `Ignored` via `manager.rs:317-319`); `engine/bridge.rs:113-124` derived pair; settings picker in the WinUI pane (blind surface — verify against a Windows build) | ~4 files |
| commit path | all platforms: 羅馬字 → existing roman commit arm (mode predicate only, no new formatter arm) | touched via settings plumb |
| i18n / content | picker copy ×3 values ×5 locales | USER 拍板 |
| docs | `behavioral-invariants.md`: §42 rewritten as a USER-approved exception (one-script cells, information loss accepted, dated quote) — not "a third arrangement"; §5 carve-out for the "entries without hanzi are always kept" clause (the §34 literal now participates in the collapse) + the dispatch-side placement of the new pass; new `INVARIANT_*` for the `(roman, span)` display dedupe + the nextword one; fix `khiin-reference.md:128` (no fuzzy tier exists); `mainstream-ime-comparison.md` correction topic row when the future 校正 proposal lands | |

## 21. Dogfood acceptance draft (Part II — final shape)

- TL mode, 候選詞顯示=羅馬字: type `taigi` → strip cells show **roman only** (`tâi-gí`, `tâi`, …), single line, no subtitle; scroll, expand chevron, expanded overlay grid, S6 index-0 keycap highlight, toolbar — **all behave exactly as today**.
- Tone-optional affordance intact: toneless `si` → the tone family surfaces as distinct roman-only cells (sī / sî / sí / si …); `hoogua` → `hōo--guá` dict form at slot 0 (S9).
- Same-roman collapse: `tsia̍h` homophones (食/𤆬) → **one** `tsia̍h` cell (vs two rows in 並排); 重's two readings tîng / tāng → **two** cells (different roman, #7 intact); §34 literal `tâi` next to dict `台/tâi` → **one** `tâi` cell, literal keeps slot 0 (pins the post-prepend placement of the dedupe — two `tâi` cells = the pass ran inside `assemble_candidates`).
- Custom entry `tsia̍h/X` sharing its roman with dict 食: visible (custom-first survivor) or collapsed — assert whichever Q9′ decided; a roman-only custom entry with the same roman as a dict row → one cell.
- NextWord: learn `guá → 食 (tsia̍h)` and `guá → 𤆬 (tsia̍h)` in 並排, switch to 羅馬字, commit `guá` → prediction strip shows **one** `tsia̍h` (Q20); switch back → two predictions with hanji again.
- Tap → document receives roman; auto-space + S10 fire (roman-ish predicate); POJ mode renders POJ (`goá`); `xyz2` garbage → literal-only, preedit verbatim (§10.2).
- 詞頻/詞關聯: commit rows identical to today's roman-first commits (Tab3 viewers; same rows climb after switching back to 並排); collapsed-group commit learns the surviving row's identity (Q9′).
- 文/A key hidden/inert per Q11; swap state restored on return to 並排; 括號標註 UI per Q17 — committed text never grows brackets in this mode.
- macOS: all three panel layouts show roman-only cells; **Space on a roman-only cell is `.ignored`** (#610 existing one-script path — regression-pin it); full-width punctuation off (roman output).
- Windows: candidate window shows roman-only cells; **Space on a roman-only cell = `Ignored`** (same pin as macOS, `manager.rs:317-319`); bare `` ` `` swap shortcut per Q11 (no-op, hint or silent); behaviour otherwise unchanged.
- TPS layout with 羅馬字 selected → unchanged TPS behaviour (hanji cells, S23-S25 unaffected); English mode path untouched; back to TL → mode resumes.
- Switching 並排 ↔ 合用 ↔ 羅馬字 live: strip re-renders, no stale layout, no width jump crashes (cells narrower, packers re-measure).
