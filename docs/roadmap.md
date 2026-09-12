# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `release trains`
> **Status**: Active
> **Last updated**: 2026-09-13 (desktop 3.6.x sections collapsed into `docs/reports/desktop-3.6.x-design-notes.md`; repository-size record retired — rationale + timings in `docs/architecture/build-artifacts.md`; released-versions index through mobile / desktop 3.6.8)

---

## Summary

- **Forward-looking work items only.** Shipped detail lives in `docs/releases/<version>/plan.md` + `changelog/<version>.md` + Claude auto-memory.
- **Active**: Telex tone-1/4 keys design (USER 2026-09-11「之後的版本再處理」). Merged desktop 3.6.x items below await dogfood only.
- **No open deferred TODO**: the keyboard theme picker (the last 2026-06-01 candidate) shipped in v3.6.2; the one design-locked, unscheduled item is 變換後羅馬字 commit (§ Out of scope / deferred).
- **Release scope / timing / tag is user-gated** per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].

---

## Active / In-flight items

kautian subcollections (腔調 + 姓名附錄 toggles + 語音差異 詞級擴展) — 5 phases MERGED, shipped **v3.6.0** (#354-#358).

---

### Desktop custom fonts — let the user add their own typeface (3.6.8, USER-scoped 2026-09-08)

**Status**: MERGED `e7d217ab` (#16) — macOS and Windows both; shipped in desktop v3.6.8. Awaiting real-device dogfood (S30 + S31).
Full design (grounded in code), rounds, the three UI tries, and dogfood text: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop custom fonts.

---

### Desktop installed typefaces — the fonts the OS already has, listed and selectable (USER-scoped 2026-09-11)

**Status**: MERGED — PR1 macOS #45 `0c3ff680`, PR2 Windows `6c06302f`. Dogfood: S42 PASS 2026-09-11; S43 PASS 2026-09-11 (project memory).
Full design (grounded in code), rounds and dogfood text: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop installed typefaces.

---

### Desktop Telex tone keys + candidate-window toggle (USER-scoped 2026-09-08)

**Status**: all rounds MERGED 2026-09-09 — P1 #17 `6888be67`, P2 #18 `cbee26d1`, P3 #19 `447154ea`, P4 #20 `938994fa`, guide P5 #21 `544d77a2`, P6 #22. Dogfood pending: S32 + S33 + S34.
Full design, rounds, the Telex-guide follow-up and dogfood text: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop Telex. The tone-1/4 follow-up below stays here as the live design record.

#### Follow-up: Telex keys for tone 1 and tone 4 (USER-decided 2026-09-11, revised 2026-09-12, NOT scheduled)

A user reported that Telex has no key for tone 1 or tone 4. USER 2026-09-11: 「先寫成文件,之後的版本
再處理」 — design recorded here, no round opened; the USER schedules it.

**Why a key at all.** Tones 1 / 4 are unmarked, so the raw buffer `tai` already reads as tone 1.
The key is an explicit pin: it narrows candidates (`tai` matches every tone, `tai1` only tone 1)
and ends a syllable in continuous input, exactly what the digits `1` / `4` do in the Standard
scheme. The buffer stays numeric-tone (`tai1`, `sit4`), so nothing downstream changes.

**Design (revised 2026-09-12).** A user proposed pairing the tones by coda: 「第一調 kap 第四調
ē-tàng 用同一个位。第 8 調會當 kap 第二調用同一个位」. USER 2026-09-12: 「先記錄」. The pairing is
phonotactically airtight: a checked syllable (coda `p / t / k / h`) can only carry tone 4 or 8, an
unchecked syllable can only carry 1 / 2 / 3 / 5 / 7 / 9, so one key never has to choose. This
supersedes the 2026-09-11 two-key design (`c` = 1, `r` = 4): `r` is no longer touched, so the
`ir` / `er` dialect-final gate and its twelve-final fixture are not needed.

| Key | Tail before caret | Meaning |
|---|---|---|
| `c` | empty, ends in `-`, or already ends in a tone digit | literal `c` — POJ `chia`, `tai5chia`, `taifchia` still type |
| `c` | ends in a stop coda `p` / `t` / `k` / `h` | append `4` (`sit` → `sit4`, `irk` → `irk4`) |
| `c` | any other complete syllable | append `1` (`tai` → `tai1`) |
| `v` | ends in a stop coda | append `8` (`tit` → `tit8`) — today `x` |
| `v` | any other complete syllable | append `2` (unchanged) |
| `y d w q` | | tone 3 5 7 9, unchanged |
| `x` | | **freed** |

The residual ambiguity is `c` alone: `tai` + `c` meant as the start of POJ `chia` pins `tai1`
instead — type `z` (already `ch` / `chh` in Telex) or tone the previous syllable first, which Telex
asks for anyway.

**Open, USER-decided when the round opens**: (1) what the freed `x` (and the untouched `r`) carry —
tone 6 (dialect; dropped in 2026-09-08 for lack of a letter), a single `tsh` key, or nothing yet;
(2) whether `x` stays an alias for tone 8 for a while, since `x` = 8 shipped in desktop 3.6.8.

**Rejected**: `;` / `'` (free in Telex since digits pick slots, but not letters — inconsistent with
the other tone keys); dropping the `ir` / `er` finals (Core Principle #3).

**When opened**: feature round. Engine `telex.rs` (`c` in `TELEX_KEYS`, the coda gate shared by
`c` and `v`, tests covering both codas per `taigi-incidents.md` § Trace before assert), `TelexKey`
proto unchanged; macOS + Windows classifiers add `c` → `.telexKey`; Telex guide rows + i18n; S32
gains `taic` → tai1, `sitc` → sit4, `titv` → tit8, `chia` unchanged.

---

### Desktop symbol picker (USER-scoped 2026-09-09)

**Status**: all rounds MERGED 2026-09-09 — P1 macOS #26 `843e3453`, P2 Windows #27 `d5a8940b`, P3 flat list #28 `7245a5b2`. Dogfood pending: S36 (both platforms).
Full design and rounds: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop symbol picker.

---

### Desktop composing caret — move inside the typed romanization (USER-scoped 2026-09-09)

**Status**: all rounds MERGED 2026-09-09 — P1 engine #29 `64e6b0b7`, P2 macOS #30 `0f3a7933`, P3 Windows #31 `3a273bff`. Dogfood pending: S37 (both platforms).
Full design and rounds: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop composing caret.

### Desktop ⇧ + slot key — the 漢羅 commit aimed at a slot (USER-scoped 2026-09-10)

**Status**: MERGED 2026-09-10 — #35 `4db4aa92` (macOS + Windows in one PR). Dogfood pending: S38 (both platforms).
Full design: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop ⇧ + slot key.

### Desktop 快速齒 pane — three title-less blocks (USER-scoped 2026-09-10)

**Status**: MERGED 2026-09-10 — #36 `db54a8d5` (macOS + Windows in one PR). Dogfood pending: both platforms (no `Sn` item).
Audit outcome, rejected alternatives (do not re-propose) and gate note: [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) § Desktop 快速齒 pane.

---

## Released versions index

Newest first. Two trains since 2026-09: mobile `mobile-x.y.z` (iOS + Android) and desktop `desktop-x.y.z` (macOS + Windows; earlier desktop tags `macos-v*` / `windows-v*` live in the website repo). Links: release notes (`changelog/`) + detailed plan archive where one exists. Authoritative ship-date list: memory `project_released_versions.md`.

| Version | Ship date | Release notes | Detailed plan archive |
|---|---|---|---|
| mobile v3.6.8 | 2026-09-12 (`mobile-3.6.8` @ `3a399505`) | [`changelog/v3.6.8.md`](../changelog/v3.6.8.md) | — (候選詞顯示 picker, 顯示當咧拍的字 default on, POJ tone placement + `o͘ⁿ`, auto-space follows commit, Android strip lag; mobile skips v3.6.6/v3.6.7) |
| desktop v3.6.8 | 2026-09-11 (`desktop-3.6.8` @ `440171d2`, same-version overwrite ×2) | [`changelog/desktop-v3.6.8.md`](../changelog/desktop-v3.6.8.md) | [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) — Telex, candidate-window toggle, symbol picker, composing caret, ⇧+slot, custom + installed fonts, 快速齒 blocks |
| desktop v3.6.7 | 2026-09-04 (macOS first 2026-09-03; assets overwritten several times through 2026-09-05; tags live in the website repo) | [`changelog/desktop-v3.6.7.md`](../changelog/desktop-v3.6.7.md) | — (first Windows release; POJ `au` tone placement; 候選詞顯示 picker on desktop) |
| v3.6.6 | 2026-08-28 (macOS-only package; no tag in this repo) | [`changelog/v3.6.6.md`](../changelog/v3.6.6.md) | — (letter-key candidate selection, Caps Lock ABC, in-app update download; iOS KeyboardKit 10.9.0) |
| v3.6.5 | 2026-08-28 (`v3.6.5` @ `84304d82`) | [`changelog/v3.6.5.md`](../changelog/v3.6.5.md) | — (macOS first release; TPS + candidate-accuracy round; §40 NextWord contract; Android 11 floor) |
| v3.6.4 | 2026-08-07 (`v3.6.4` @ `ed38499f`) | [`changelog/v3.6.4.md`](../changelog/v3.6.4.md) | — (App UI i18n — five display languages) |
| v3.6.3 | 2026-06-20 (`v3.6.3` @ `acea9a8f`) | [`changelog/v3.6.3.md`](../changelog/v3.6.3.md) | — (TPS fixes: explicit tone, tone 9, `ir`; single-initial input; Android autocorrect / vibration / rich-editor backspace) |
| v3.6.2 | 2026-06-12 (`v3.6.2` @ `c550e100`) | [`changelog/v3.6.2.md`](../changelog/v3.6.2.md) | — (keyboard theme picker + custom theme editor; 顯示羅馬字 toggle) |
| v3.6.1 | 2026-06-06 (`v3.6.1` @ `d1259966`) | [`changelog/v3.6.1.md`](../changelog/v3.6.1.md) | [`docs/reports/2026-06-03-user-data-cross-mode-audit.md`](reports/2026-06-03-user-data-cross-mode-audit.md) — cross-mode user-data consistency R1–R7, 漢羅 literal candidate, backup exclusion |
| v3.6.0 | 2026-05-31 (`b782205c`) | [`changelog/v3.6.0.md`](../changelog/v3.6.0.md) | — (kautian subcoll + dev supplement + source-toggle filtering + explicit-tone fix) |
| v3.5.9 | 2026-05-29 (`3c8bec16`) | [`changelog/v3.5.9.md`](../changelog/v3.5.9.md) | — (TPS 三索引 + Tier-A/B refactor; design memo `project_v359_d_tps_triindex_plan.md`) |
| v3.5.8 | 2026-05-20 (`61df3028`) | [`changelog/v3.5.8.md`](../changelog/v3.5.8.md) | [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) — Phase 0-9 + 整句 lattice + walker S1-S9 + continuous-compound-hyphen fix |
| v3.5.7 | 2026-05-08 | [`changelog/v3.5.7.md`](../changelog/v3.5.7.md) | — |
| v3.5.6 | 2026-04-27 | [`changelog/v3.5.6.md`](../changelog/v3.5.6.md) | — |
| v3.5.5 | 2026-04-12 | [`changelog/v3.5.5.md`](../changelog/v3.5.5.md) | — |
| v3.5.3 | 2026-03-22 | [`changelog/v3.5.3.md`](../changelog/v3.5.3.md) | — |
| v3.5.2 | 2026-03-08 | [`changelog/v3.5.2.md`](../changelog/v3.5.2.md) | — |
| v3.5.1 | 2026-02-25 | [`changelog/v3.5.1.md`](../changelog/v3.5.1.md) | — |
| v3.5.0 | 2026-02-12 | [`changelog/v3.5.0.md`](../changelog/v3.5.0.md) | — |
| v3.4.x | 2025-2026 | [`changelog/v3.4.*.md`](../changelog/) | — |
| v3.3.x | 2025 | [`changelog/v3.3.*.md`](../changelog/) | — |

Detailed plan archives are added retroactively only when source material exists; older versions remain release-notes-only.

---

## Out of scope / deferred (truly forward-looking)

Forward-looking candidates only, NOT items already shipped. (v3.5.8-era items that read like candidates but shipped — `整句 lattice + walker`, `continuous compound-hyphen`, `Phase 9 user-freq plumb` — live in [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md).)

### 變換後羅馬字 commit — segment + numeric-tone→diacritic on Enter

**Status**: design locked (Option A), NOT implemented, USER-gated — pre-arranged 2026-06-29 at USER request to minimize impl-time effort (USER 「預先安排好v3.6.5的項目，減少之後實作的effort」); v3.6.5 shipped 2026-08-28 without it and no later version is assigned. Round still gated on USER UX confirm + Codex pre-impl. **Full design + code seams + Codex prompt**: memory `project_roman_convert_on_commit.md`.

In TL/POJ, Enter should commit the **converted** romanization — multi-syllable segmentation + numeric tone → tone-diacritic (`suann2ting3` → `suán-tìng`), matching PhahTaigi. Today single (`suann2`→`suán`) and hyphenated (`tai5-gi2`→`tâi-gí`) convert; un-hyphenated multi-syllable stays verbatim per §10.2. Requester = Kisaragi Hiu (same person who drove the S22/§34 literal-roman candidate). **Locked design = Option A**: preedit stays verbatim while typing (§10.2 WYSIWYG), Enter commits converted, raw output stays free via tapping the existing verbatim strip-#0 literal candidate (no new UX element). Engine work = deterministic tone-digit pre-segmentation in `phonetics::canonical_tl_form` (digit ends a syllable → no FST inventory needed; fallback verbatim on ambiguous toneless/invalid input). Platform work = Enter commits the composition's `canonical_tl` instead of raw preedit (locate each platform's return-key composition handler at impl). Touches invariants §10.2 / §17 / §34 (S22) / Core Principle #7 — feature round updates them + adds a cross-platform `INVARIANT_*` test in the same PR. Engine-only fix covers iOS+Android; `make build` (no `make dict`).

**Gmail report pass (USER 2026-06-29)**: the dogfood-confirmation pass over the then-open Gmail user-report bugs was completed in 2026-08 (memory `MEMORY.md` § Bug reports: batch of 9 closed — 5 fixed, 4 not reproducible). New reports follow `/bug-triage` per incident (Core Principle #4).

## Per-round gates (process invariants, project-wide)

Apply to every coding round regardless of release. Authoritative source: `~/.claude/rules/round-workflow.md`.

Project-specific additions only (branching, sandwich, test scope, admin tier live in that rule):

- Cross-platform parity-correction rounds merge both platforms in lockstep.
- iOS `pbxproj` is user-only (`.claude/rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance.

---

## Closed phases / shipped audits

- **v3.6.1 user-data key consistency across input modes** — CLOSED / shipped: rounds R1–R7 MERGED 2026-06-03/04 (#382–#388; association recall + canonical-TL commit + custom words cross-mode + Android cap parity + `(hanji, tl)` frequency key + SQLite hygiene + backup exclusion). Dogfood items S11–S16. Triple index kept. Full audit: [`docs/reports/2026-06-03-user-data-cross-mode-audit.md`](reports/2026-06-03-user-data-cross-mode-audit.md).
- **App UI i18n — multi-language** (漢字 / English / 日本語 / Tâi-lô / Pe̍h-ōe-jī + Automatic) — SHIPPED; all five display languages in the production picker. Open: TL/POJ prose proofreading by the USER (data-only). Outcome: `.claude/rules/i18n.md`, `behavioral-invariants.md` §37–39, `system-overview.md` §3 (`make i18n`).
- **Keyboard theme picker** (swipe gallery + custom theme) — SHIPPED v3.6.2: iOS #400-411, Android port #412-#418. Current-state reference: [`docs/ui/theme.md`](ui/theme.md).
- **Android UI modernization** (Compose M3 chrome/overlay) — DONE 2026-05-30 (#362 / #364 / #365). 3 leaf overlays (Symbol/Layout/Candidate) View→Compose M3 over `KeyboardChromeColors`; keys stay custom-draw; `InputView`/window kept View (IME-dismiss bug zone). Memory `project_android_compose_modernization.md`.
- **v3.5.9 D = TPS 三索引** — SHIPPED, tagged `3c8bec16` 2026-05-29. `tps:` FST family parallel to `tl:` / `poj:`; mode-axis (Input + Key + FST) now three-layer symmetric. Retired `is_tps` short-circuit (`dispatch.rs`/`continuous.rs`), `tps_or_mapped_to_er` runtime branch (`search.rs`), `tps_to_tl` canonicalize chain (`classification.rs`). 6 PR (#334-#340, C-0/C-1/C-3a/C-3b/C-4/C-5).
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (#212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (#227-#231).
- v3.5.8 連續輸入 — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for full plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in Active / In-flight items. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry in Closed phases. -->
