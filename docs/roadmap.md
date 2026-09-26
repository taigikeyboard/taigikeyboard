# TaigiKeyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `release trains`
> **Status**: Active
> **Last updated**: 2026-09-26 (Active items all merged — collapsed into Closed phases; design bodies frozen in `docs/reports/2026-09-26-shipped-roadmap-design-notes.md`)

---

## Summary

- **Forward-looking work items only.** Shipped detail lives in `docs/releases/<version>/plan.md` + `changelog/mobile-<version>.md` + Claude auto-memory.
- **Active**: none — every scoped item has merged; pending dogfood is tracked in `docs/architecture/dogfood-checklist.md`.
- **No open deferred TODO**: the keyboard theme picker (the last 2026-06-01 candidate) shipped in v3.6.2; the one design-locked, unscheduled item is the converted-romanization commit (§ Out of scope / deferred).
- **Release scope / timing / tag is user-gated** per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].

---

## Active / In-flight items

None — every item scoped through 2026-09-26 has merged (see Closed phases). Pending dogfood: `docs/architecture/dogfood-checklist.md`.

---

## Released versions index

Newest first. Two trains since 2026-09: mobile `mobile-x.y.z` (iOS + Android) and desktop `desktop-x.y.z` (macOS + Windows + Linux; earlier desktop tags `macos-v*` / `windows-v*` live in the website repo). Links: release notes (`changelog/`) + detailed plan archive where one exists. Authoritative ship-date list: memory `project_released_versions.md`.

| Version | Ship date | Release notes | Detailed plan archive |
|---|---|---|---|
| desktop v3.6.10 | 2026-09-24 (`desktop-3.6.10` @ `f6f00b47`, Linux-only patch; macOS / Windows stay on 3.6.9) | [`changelog/desktop-v3.6.10.md`](../changelog/desktop-v3.6.10.md) | — (Fcitx5 selection keys, vertical-window arrow keys, Linux Fonts pane removed) |
| desktop v3.6.9 | 2026-09-23 (`desktop-3.6.9` @ `6523379e`, re-cut; first Linux packages uploaded 2026-09-24) | [`changelog/desktop-v3.6.9.md`](../changelog/desktop-v3.6.9.md) | [`docs/reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) — learned phrases, Telex tone 1 / 4, input-source menu |
| mobile v3.6.8 | 2026-09-12, re-cut 2026-09-14 (`mobile-3.6.8` @ `4022956b`, first cut `3a399505`) | [`changelog/mobile-v3.6.8.md`](../changelog/mobile-v3.6.8.md) | — (Candidate Display picker, Show Typed Text default on, POJ tone placement + `o͘ⁿ`, auto-space follows commit, Android strip lag; mobile skips v3.6.6/v3.6.7) |
| desktop v3.6.8 | 2026-09-11 (`desktop-3.6.8` @ `440171d2`, same-version overwrite ×2) | [`changelog/desktop-v3.6.8.md`](../changelog/desktop-v3.6.8.md) | [`docs/reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md) — Telex, candidate-window toggle, symbol picker, composing caret, ⇧+slot, custom + installed fonts, Shortcuts blocks |
| desktop v3.6.7 | 2026-09-04 (macOS first 2026-09-03; assets overwritten several times through 2026-09-05; tags live in the website repo) | [`changelog/desktop-v3.6.7.md`](../changelog/desktop-v3.6.7.md) | — (first Windows release; POJ `au` tone placement; Candidate Display picker on desktop) |
| v3.6.6 | 2026-08-28 (macOS-only package; no tag in this repo) | [`changelog/desktop-v3.6.6.md`](../changelog/desktop-v3.6.6.md) | — (letter-key candidate selection, Caps Lock ABC, in-app update download; iOS KeyboardKit 10.9.0) |
| v3.6.5 | 2026-08-28 (`v3.6.5` @ `84304d82`) | [`changelog/mobile-v3.6.5.md`](../changelog/mobile-v3.6.5.md) | — (macOS first release; TPS + candidate-accuracy round; §40 NextWord contract; Android 11 floor) |
| v3.6.4 | 2026-08-07 (`v3.6.4` @ `ed38499f`) | [`changelog/mobile-v3.6.4.md`](../changelog/mobile-v3.6.4.md) | — (App UI i18n — five display languages) |
| v3.6.3 | 2026-06-20 (`v3.6.3` @ `acea9a8f`) | [`changelog/mobile-v3.6.3.md`](../changelog/mobile-v3.6.3.md) | — (TPS fixes: explicit tone, tone 9, `ir`; single-initial input; Android autocorrect / vibration / rich-editor backspace) |
| v3.6.2 | 2026-06-12 (`v3.6.2` @ `c550e100`) | [`changelog/mobile-v3.6.2.md`](../changelog/mobile-v3.6.2.md) | — (keyboard theme picker + custom theme editor; Show Romanization toggle) |
| v3.6.1 | 2026-06-06 (`v3.6.1` @ `d1259966`) | [`changelog/mobile-v3.6.1.md`](../changelog/mobile-v3.6.1.md) | [`docs/reports/2026-06-03-user-data-cross-mode-audit.md`](reports/2026-06-03-user-data-cross-mode-audit.md) — cross-mode user-data consistency R1–R7, Hanji-with-romanization literal candidate, backup exclusion |
| v3.6.0 | 2026-05-31 (`b782205c`) | [`changelog/mobile-v3.6.0.md`](../changelog/mobile-v3.6.0.md) | — (kautian subcoll + dev supplement + source-toggle filtering + explicit-tone fix) |
| v3.5.9 | 2026-05-29 (`3c8bec16`) | [`changelog/mobile-v3.5.9.md`](../changelog/mobile-v3.5.9.md) | — (TPS tri-index + Tier-A/B refactor; design memo `project_v359_d_tps_triindex_plan.md`) |
| v3.5.8 | 2026-05-20 (`61df3028`) | [`changelog/mobile-v3.5.8.md`](../changelog/mobile-v3.5.8.md) | [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) — Phase 0-9 + whole-sentence lattice + walker S1-S9 + continuous-compound-hyphen fix |
| v3.5.7 | 2026-05-08 | [`changelog/mobile-v3.5.7.md`](../changelog/mobile-v3.5.7.md) | — |
| v3.5.6 | 2026-04-27 | [`changelog/mobile-v3.5.6.md`](../changelog/mobile-v3.5.6.md) | — |
| v3.5.5 | 2026-04-12 | [`changelog/mobile-v3.5.5.md`](../changelog/mobile-v3.5.5.md) | — |
| v3.5.3 | 2026-03-22 | [`changelog/mobile-v3.5.3.md`](../changelog/mobile-v3.5.3.md) | — |
| v3.5.2 | 2026-03-08 | [`changelog/mobile-v3.5.2.md`](../changelog/mobile-v3.5.2.md) | — |
| v3.5.1 | 2026-02-25 | [`changelog/mobile-v3.5.1.md`](../changelog/mobile-v3.5.1.md) | — |
| v3.5.0 | 2026-02-12 | [`changelog/mobile-v3.5.0.md`](../changelog/mobile-v3.5.0.md) | — |
| v3.4.x | 2025-2026 | [`changelog/mobile-v3.4.*.md`](../changelog/) | — |
| v3.3.x | 2025 | [`changelog/mobile-v3.3.*.md`](../changelog/) | — |

Detailed plan archives are added retroactively only when source material exists; older versions remain release-notes-only.

---

## Out of scope / deferred (truly forward-looking)

Forward-looking candidates only, NOT items already shipped. (v3.5.8-era items that read like candidates but shipped — `whole-sentence lattice + walker`, `continuous compound-hyphen`, `Phase 9 user-freq plumb` — live in [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md).)

### Converted-romanization commit — segment + numeric-tone→diacritic on Enter

**Status**: design locked (Option A), NOT implemented, USER-gated — pre-arranged 2026-06-29 at USER request to minimize impl-time effort (USER: "arrange the v3.6.5 items in advance to reduce the implementation effort later"); v3.6.5 shipped 2026-08-28 without it and no later version is assigned. Round still gated on USER UX confirm + Codex pre-impl. **Full design + code seams + Codex prompt**: memory `project_roman_convert_on_commit.md`.

In TL/POJ, Enter should commit the **converted** romanization — multi-syllable segmentation + numeric tone → tone-diacritic (`suann2ting3` → `suán-tìng`), matching PhahTaigi. Today single (`suann2`→`suán`) and hyphenated (`tai5-gi2`→`tâi-gí`) convert; un-hyphenated multi-syllable stays verbatim per §10.2. Requester = Kisaragi Hiu (same person who drove the S22/§34 literal-roman candidate). **Locked design = Option A**: preedit stays verbatim while typing (§10.2 WYSIWYG), Enter commits converted, raw output stays free via tapping the existing verbatim strip-#0 literal candidate (no new UX element). Engine work = deterministic tone-digit pre-segmentation in `phonetics::canonical_tl_form` (digit ends a syllable → no FST inventory needed; fallback verbatim on ambiguous toneless/invalid input). Platform work = Enter commits the composition's `canonical_tl` instead of raw preedit (locate each platform's return-key composition handler at impl). Touches invariants §10.2 / §17 / §34 (S22) / Core Principle #7 — feature round updates them + adds a cross-platform `INVARIANT_*` test in the same PR. Engine-only fix covers iOS+Android; `make build` (no `make dict`).

**Gmail report pass (USER 2026-06-29)**: the dogfood-confirmation pass over the then-open Gmail user-report bugs was completed in 2026-08 (memory `MEMORY.md` § Bug reports: batch of 9 closed — 5 fixed, 4 not reproducible). New reports follow `/bug-triage` per incident (Core Principle #4).

### TL mode (臺羅模式) — tone key commits without a candidate window (MOE parity)

**Status**: research only, NOT implemented, **low priority** (USER 2026-09-21: "this feature is neither urgent nor important, and demand is low, so its priority is lower"). Community request: in pure-romanization typing the tone key should write the syllable at once, as the MOE Mac IME's 臺羅模式 does (`tai5` → `tâi`, no Enter, no window). Real on the desktops only — macOS / Windows Space is `.ignored` under Candidate Display = Romanization Only, so every word costs an Enter; mobile Space already commits. Marked text itself stays (rewrite-on-tone rejected). Two options costed — A: desktop Space commits when it has no alternate script (~30 LOC each side); B: a real TL mode (tone key commits, no window, hand-typed hyphens). Open USER decisions (where the mode lives, tone 1/4 ending, mobile parity) + MOE behaviour still to verify on a Mac. Full write-up: [`reports/2026-09-21-taile-mode-tone-commit.md`](reports/2026-09-21-taile-mode-tone-commit.md).

## Per-round gates (process invariants, project-wide)

Apply to every coding round regardless of release. Authoritative source: `~/.claude/rules/round-workflow.md`.

Project-specific additions only (branching, sandwich, test scope, admin tier live in that rule):

- Cross-platform parity-correction rounds merge both platforms in lockstep.
- iOS `pbxproj` is user-only (`.claude/rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance.

---

## Closed phases / shipped audits

- **User data in the engine** — P0–P9d MERGED 2026-09-26 (#219–#237): the four user-data SQLite stores are engine-owned (`engine/userdata`). Design + PR table: [`architecture/user-data-engine-roadmap.md`](architecture/user-data-engine-roadmap.md). Device dogfood pending (iOS first look OK).
- **Identical desktop menus; Linux update check added then removed** — phases 1–5 MERGED 2026-09-25 (#175–#178, site #20); Linux half reversed the same day (#193, no update check on Linux). S77. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Linux update check.
- **Learned phrases** — #109–#113 MERGED 2026-09-20; own store PR-A–D #125–#128 MERGED 2026-09-21; store engine-owned since user-data P3c. S62. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Learned phrases.
- **Mobile custom theme — one background surface, gradient direction, photo background** — A–D #90–#93 MERGED 2026-09-20 (+ follow-up E). S54 / S55. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Mobile custom theme; current state [`ui/theme.md`](ui/theme.md).
- **Desktop Telex keys for tone 1 / 4** — #98 `17850f17` MERGED 2026-09-19. S57. Design: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Desktop Telex keys.
- **Desktop input-source menu — global shortcut rows** — #100 `f763d8bf` MERGED 2026-09-20. S59. Notes: [`reports/2026-09-26-shipped-roadmap-design-notes.md`](reports/2026-09-26-shipped-roadmap-design-notes.md) § Desktop input-source menu.
- **Desktop 3.6.8 items** — custom fonts #16, installed typefaces #45 (S42 / S43 PASS), Telex keys + candidate-window toggle #17–#22, symbol picker #26–#28, composing caret #29–#31, ⇧ + slot key #35, Shortcuts pane #36; shipped in desktop v3.6.8. Dogfood S30–S38. Design: [`reports/desktop-3.6.x-design-notes.md`](reports/desktop-3.6.x-design-notes.md).
- **kautian subcollections** (accent + Surname Appendix toggles + pronunciation-difference word-level extension) — 5 phases MERGED, shipped **v3.6.0** (#354-#358).
- **v3.6.1 user-data key consistency across input modes** — CLOSED / shipped: rounds R1–R7 MERGED 2026-06-03/04 (#382–#388; association recall + canonical-TL commit + custom words cross-mode + Android cap parity + `(hanji, tl)` frequency key + SQLite hygiene + backup exclusion). Dogfood items S11–S16. Triple index kept. Full audit: [`docs/reports/2026-06-03-user-data-cross-mode-audit.md`](reports/2026-06-03-user-data-cross-mode-audit.md).
- **App UI i18n — multi-language** (Hanji / English / Japanese / Tâi-lô / Pe̍h-ōe-jī + Automatic) — SHIPPED; all five display languages in the production picker. Open: TL/POJ prose proofreading by the USER (data-only). Outcome: `.claude/rules/i18n.md`, `behavioral-invariants.md` §37–39, `system-overview.md` §3 (`make i18n`).
- **Keyboard theme picker** (swipe gallery + custom theme) — SHIPPED v3.6.2: iOS #400-411, Android port #412-#418. Current-state reference: [`docs/ui/theme.md`](ui/theme.md).
- **Android UI modernization** (Compose M3 chrome/overlay) — DONE 2026-05-30 (#362 / #364 / #365). 3 leaf overlays (Symbol/Layout/Candidate) View→Compose M3 over `KeyboardChromeColors`; keys stay custom-draw; `InputView`/window kept View (IME-dismiss bug zone). Memory `project_android_compose_modernization.md`.
- **v3.5.9 D = TPS tri-index** — SHIPPED, tagged `3c8bec16` 2026-05-29. `tps:` FST family parallel to `tl:` / `poj:`; mode-axis (Input + Key + FST) now three-layer symmetric. Retired `is_tps` short-circuit (`dispatch.rs`/`continuous.rs`), `tps_or_mapped_to_er` runtime branch (`search.rs`), `tps_to_tl` canonicalize chain (`classification.rs`). 6 PR (#334-#340, C-0/C-1/C-3a/C-3b/C-4/C-5).
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (#212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (#227-#231).
- v3.5.8 continuous input — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for full plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in Active / In-flight items. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry in Closed phases. -->
