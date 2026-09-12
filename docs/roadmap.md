# Taigi Keyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `released versions`, `deferred items`
> **Status**: Active
> **Last updated**: 2026-09-13 (merged desktop 3.6.x sections collapsed; design bodies frozen in `docs/reports/desktop-3.6.x-design-notes.md`)

---

## Summary

- **Forward-looking work items only.** Shipped detail lives in `docs/releases/<version>/plan.md` + `changelog/<version>.md` + Claude auto-memory.
- **Active**: Telex tone-1/4 keys design (USER 2026-09-11「之後的版本再處理」). Merged desktop 3.6.x items below await dogfood only.
- **Deferred TODO (1)**: keyboard theme picker. All other prior candidates closed 2026-06-01 (USER).
- **Release scope / timing / tag is user-gated** per [`~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope].

---

## Active / In-flight items

kautian subcollections (腔調 + 姓名附錄 toggles + 語音差異 詞級擴展) — 5 phases MERGED, shipped **v3.6.0** (#354-#358).

---

### Repository size — stop committing build artifacts (2026-09-07, USER-approved)

**Status**: SUPERSEDED by the USER's 2026-09-07 decision — dictionaries and fonts stay committed (now single-sourced at `dictionaries/` and `fonts/font/`, PR #14 `f4e0fb20`); only the engine binaries became `make build` output (#1–#3). Phase 4 not doing. Kept as the design record.

The repository is **753 MB**. Source code is 61.5 MB of it; the rest is build
output committed on every rebuild. A trial `git filter-repo` measured what is
recoverable:


Two separable problems. Gitignoring artifacts **stops growth but shrinks
nothing** — every past revision stays in the pack. Only a history rewrite
reclaims the 536 MB.

**What is generated, and by what** (verified against the pipeline, not assumed):

| Kind | Tracked | Generator |
| --- | --- | --- |
| Engine binaries — `.a` / `.so` / xcframework | 46.7 MB | `make build` |
| `dictionary/output/*` | 58.6 MB | `make dict` |
| Platform dictionary copies (×4) | 69.7 MB | `make dict` → `dictionary/build/deploy.sh` |
| Per-source `sources/*/data/<src>.csv` | ~30 MB | `dictionary/pipeline/context.py:104-106` |
| Font copies for Android / macOS / Windows | **118 MB** | since removed — see below |

`sources/*/data/raw/*` (~35 MB) is a real input and stays committed. So are the
typefaces, but no longer four times over: the four copies this table measured
became one shared `fonts/font/`, packaged by all four platforms, so 118 MB of
duplicate left the tree and `scripts/sync-fonts.sh` went with it.

**Cost of the change is close to zero for the maintainer.** Ignored files
survive `git checkout`, so `make dict` / `make build` run once per machine, and
after that only when their inputs change — which is what
`CLAUDE.md`'s stale-binary gate already requires. The recurring cost falls on
the GitHub-hosted Windows build, which checks out fresh. In the end phase 3 was
narrowed and that workflow gained only a `make fonts` step — itself removed when
the font trees were replaced by the single shared directory.

**Phases**

| # | Change | Outcome |
| --- | --- | --- |
| 1 | `version_snapshot.py` diffs releases against a committed 3.1 MB `dictionary/word-keys.tsv` instead of the 35 MB `dictionary/output/dictionary.csv` | **Merged** (#1) |
| 2 | macOS and Windows font trees become `make fonts` output and are ignored | **Merged** (#2) |
| 3 | iOS and macOS xcframeworks and the Android `.so` become `make build` output and are ignored; `CLAUDE.md` grows a bootstrap section | **Merged** (#3) — narrowed to the engine binaries |
| 4 | `git filter-repo` purges the artifacts from history | **Not doing** (USER 2026-09-07) |

**Phase 3 was narrowed on a diagnosis that turned out to be wrong.**
Bootstrapping a fresh clone showed `make dict` dying in `cleanup.py:201` with a
`TypeError` comparing a str to an int, which was read as the dictionary
artifacts being unreproducible. The real cause was that the test clone had no
`--recurse-submodules`: `taigi-converter/` was empty, node failed its import,
and every conversion returned an error string that the pipeline ingested as
data. With the submodule checked out, `make dict` completes on a fresh clone and
deploys to all four platforms. Both entry points now check for it.

The dictionary artifacts nonetheless stay tracked, on the USER's instruction
rather than on that reasoning: 「dictionary/ folder 都不要碰,只要 build 出來的
產物在各大平台上有清理就好了」. `dictionary/output/`, the per-platform copies and
the per-source pipeline CSVs are all still committed, and nothing under
`dictionary/` was changed.

That left phase 4 purging only the platform build artifacts — measured at
**753 MB → 609 MB**, a 19% reduction bought with another full commit-SHA
rewrite and 17 dropped commits. The 392 MB that made the rewrite worth doing is
in the dictionary artifacts, which are staying. USER dropped phase 4 rather than
pay the full cost for the remainder.

**Where this leaves the repository**: still 753 MB, but the growth is stopped —
the engine binaries and font copies that accounted for most of it are no longer
committed on every rebuild. Reopening phase 4 only makes sense after the
dictionary pipeline can rebuild from a clean checkout.

**Measured runtimes** (2026-09-07, warm machine, fresh clone with submodules):

| | Wall | Where it goes |
| --- | --- | --- |
| `make dict` | **~2 min** | `run.sh` 67 s across the nine per-source pipelines (`kautian` 26 s, `taihoa` 10 s; by stage, `extract` 15 s and `merge` 8 s dominate), then `build.sh` 53 s (`merge_csv` 16 s, `create_association_bin`+verify 13 s, `create_syllables_fst` 11 s, `create_fst` 7 s, `create_dictionary_bin`+verify 5 s, everything else under a second) |
| `make build` | **5.1 s warm** | all six steps; every cargo invocation reports `Finished` in 0.03–0.08 s against a warm target dir. A cold build compiles the engine for five targets and takes minutes — not measured here. |

208,595 raw input rows, every reading converted through the Node bridge, three
FST/mmap artifacts built and verified. Nothing pathological.

Two earlier figures in this document were wrong: `~30 s` was a guess, and the
`10 minutes` that replaced it was measured against a run that spent most of its
time on the broken-submodule path, failing one conversion per row.

A dead-weight note for whoever reopens this: history still carries ~190 MB of
pipeline layouts that no longer exist at `HEAD` (`dictionary/csv/`,
`dictionary/6_台日大辭典/`, `dictionary/5_台華線頂對照典/`). They would go in the
same pass.

**Invariant being overturned**: `.gitignore:88-92` states that the Android `.so`
and iOS xcframework are *both* committed "so a release tag carries a complete,
buildable engine on both platforms" (D9.2). That trade is being reversed: a tag
plus a reproducible `make build` replaces a tag that carries binaries.

### Desktop custom fonts — let the user add their own typeface (3.6.8, USER-scoped 2026-09-08)

**Status**: MERGED `e7d217ab` (#16) — macOS and Windows both. Awaiting real-device dogfood (S30 + S31).
**Scope**: macOS + Windows only. iOS and Android are deliberately untouched — the USER scoped this
to the desktop train.

Today the candidate-window typeface is a closed roster of five: the system face plus the four
files in the repo-root `fonts/font/`. `CandidateFontChoice` spells that roster three times over —
as a Swift enum with PostScript names (`macos/.../Candidates/CandidateFontChoice.swift:22-48`), as
a Rust enum with file names (`windows/.../settings/choices.rs:188-245`), and as DirectWrite family
names (`windows/.../ui/render.rs:50-58`). A user who wants any other face has no way in.

**The feature**: an "add a typeface" flow in the Appearance pane. The user picks a font file; the
app copies it into its own directory, reads the face name back out of it, and the file joins the
picker's roster. Added faces can be removed again.

#### Design

**A font library, copied — not a path remembered.** The chosen file is copied into
`~/Library/Application Support/TaigiKeyboard/Fonts/` (macOS) and `%APPDATA%\TaigiKeyboard\Fonts\`
(Windows), beside a small `fonts.json` index holding, per entry, the stored file name, the display
name, and the face name the font declares. The original may live on a removable volume, be deleted,
or sit somewhere a host process cannot read; the app-owned copy is the only one that is always
there. On Windows the same directory already holds `settings.json` and the user-data databases that
the TIP reads from inside every host process, so the path is proven reachable.

**The enum does not open up.** `CandidateFontChoice` gains one unit case, `custom` (raw value
`"custom"`), and a new settings key `customFontFile` names which stored file is live. The user may
keep several installed; only one is ever selected, so one key carries it. This keeps Windows'
`FontSpec`/`FormatKey` `Copy + Hash` (`windows/.../candidates/metrics.rs:23-26`) and keeps
`SettingChoice::raw` returning `&'static str`.

**Import validates by loading.** Extension in `.ttf` / `.otf` / `.ttc`, a size ceiling (the bundled
GenYoMin is already >20 MB — 64 MB), and then the real gate: the file must load and yield a face
name — `CTFontManagerCreateFontDescriptorsFromURL` on macOS, `IDWriteFontSetBuilder1::AddFontFile`
plus `GetPropertyValues(FAMILY_NAME)` on Windows. A file that fails is refused with a message and
nothing is left behind. A `.ttc` contributes its first face, matching the single-weight model the
roster already has. Name collisions get a suffix.

**Taking effect without a restart** is the real Windows work. macOS registers the copy with
`CTFontManagerRegisterFontsForURL(.process)` in the same process that draws the candidate window,
so the next window picks it up. On Windows the TIP lives in each host process and
`load_private_fonts` runs exactly once, at `RenderFactory::new` (`render.rs:294-322`). The custom
face needs its own collection, keyed by file name + mtime + the `settings.json` revision the TIP
already watches, and both `formats` and `ellipsis` caches must be dropped when that key moves —
otherwise replacing a file under the same name keeps drawing the old outlines.

**Deliberately not adopted**: weight/variable-axis selection, a rendered preview of each face,
syncing the library between machines, and any mobile counterpart. Fonts are copied for local use
only and never redistributed, so `THIRD_PARTY_LICENSES.md` is unaffected.

#### Rounds

| PR | Scope | Est. |
|---|---|---|
| P0 | This section + memory (admin tier, direct to main) | — |
| P1 | macOS whole: font library, `CandidateFontSelection`, launch-time registration, UI, i18n keys | done |
| P2 | Windows core + storage + platform: the selection type, the two-key resolver, the file half (host-tested) and the DirectWrite half | done |
| P3 | Windows render: a collection per custom face, an id per loaded resource, mtime+length as the change detector, format caches dropped with it | done |
| P4 | Windows settings window: the 字型管理 pane, `+` / `−`, the file dialog's font filter | done |

All of it landed in one PR (#16) at the USER's instruction, macOS first and Windows after.

**The UI shape took three tries** (USER 2026-09-08). A section in 外觀 with a per-row delete
button, then a sheet, then a pane for the custom fonts alone — each was rejected, and the
reason each time was the same one: choosing a typeface and managing the list are two
selections that look alike. The answer was to put the bundled roster and the user's own
typefaces in ONE list, in a 字型管理 pane of its own, where the selected row IS the typeface
in use — which is how System Settings states a list like that (聲音's output devices,
顯示器's displays). 外觀 lost its font row; each pane's reset restores the rows it shows.

#### Dogfood (new items, to be added to `docs/architecture/dogfood-checklist.md` in P1/P4)

- **S30 macOS** — add a typeface → select it → the candidate window redraws in it → delete the live
  one → falls back to the system face, all without restarting the input method.
- **S31 Windows** — add a typeface in the settings window; an **already-running** host (Notepad plus
  a WinUI app) shows it on the next candidate window. Replacing a file under the same name does not
  keep drawing the old one, and a file another host still holds refuses to be deleted with a message
  rather than silently.

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

Newest first. Links: release notes (`changelog/`) + detailed plan archive (`docs/releases/`) where one exists. Authoritative ship-date list: memory `project_released_versions.md`.

| Version | Ship date | Release notes | Detailed plan archive |
|---|---|---|---|
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

### 變換後羅馬字 commit — segment + numeric-tone→diacritic on Enter (v3.6.5)

**Status**: Design LOCKED, NOT implemented — pre-arranged 2026-06-29 at USER request to minimize impl-time effort (USER 「預先安排好v3.6.5的項目，減少之後實作的effort」; originally tagged v3.6.4 2026-06-20, moved because i18n took v3.6.4). Round still gated on USER UX confirm + Codex pre-impl. **Full design + code seams + Codex prompt**: memory `project_roman_convert_on_commit.md`.

In TL/POJ, Enter should commit the **converted** romanization — multi-syllable segmentation + numeric tone → tone-diacritic (`suann2ting3` → `suán-tìng`), matching PhahTaigi. Today single (`suann2`→`suán`) and hyphenated (`tai5-gi2`→`tâi-gí`) convert; un-hyphenated multi-syllable stays verbatim per §10.2. Requester = Kisaragi Hiu (same person who drove the S22/§34 literal-roman candidate). **Locked design = Option A**: preedit stays verbatim while typing (§10.2 WYSIWYG), Enter commits converted, raw output stays free via tapping the existing verbatim strip-#0 literal candidate (no new UX element). Engine work = deterministic tone-digit pre-segmentation in `phonetics::canonical_tl_form` (digit ends a syllable → no FST inventory needed; fallback verbatim on ambiguous toneless/invalid input). Platform work = Enter commits the composition's `canonical_tl` instead of raw preedit (locate each platform's return-key composition handler at impl). Touches invariants §10.2 / §17 / §34 (S22) / Core Principle #7 — feature round updates them + adds a cross-platform `INVARIANT_*` test in the same PR. Engine-only fix covers iOS+Android; `make build` (no `make dict`).

**Also scoped to v3.6.5 (USER 2026-06-29)**: a dogfood-confirmation pass over the 8 open Gmail user-report bugs (`/bug-triage` queue; full list + ids in memory `reference_gmail_bug_triage`). This is a verification checklist, not a commitment to fix all 8 — already-fixed-on-current-build reports get labeled `已修復`, still-reproducing ones become user-gated per-incident fix rounds (Core Principle #4).

## Per-round gates (process invariants, project-wide)

Apply to every coding round regardless of release. Authoritative source: `~/.claude/rules/round-workflow.md`.

Project-specific additions only (branching, sandwich, test scope, admin tier live in that rule):

- Cross-platform parity-correction rounds merge both platforms in lockstep.
- iOS `pbxproj` is user-only (`.claude/rules/ios-guidelines.md`); Android Gradle is editable.
- Engine slices require S0 golden-diff EMPTY acceptance.

---

## Closed phases / shipped audits

- **v3.6.1 user-data key consistency across input modes** — CLOSED / shipped: rounds R1–R7 MERGED 2026-06-03/04 (#382–#388; association recall + canonical-TL commit + custom words cross-mode + Android cap parity + `(hanji, tl)` frequency key + SQLite hygiene + backup exclusion). Dogfood items S11–S16. Triple index kept. Full audit: [`docs/reports/2026-06-03-user-data-cross-mode-audit.md`](reports/2026-06-03-user-data-cross-mode-audit.md).
- **App UI i18n — multi-language** (漢字 / English / 日本語 / Tâi-lô / Pe̍h-ōe-jī + Automatic) — SHIPPED; all five display languages in the production picker. Open: TL/POJ prose proofreading by the USER (data-only). Full plan: [`docs/architecture/i18n-multilang-plan.md`](architecture/i18n-multilang-plan.md).
- **Keyboard theme picker** (swipe gallery + custom theme) — SHIPPED v3.6.2: iOS #400-411, Android port #412-#418. Current-state reference: [`docs/ui/theme.md`](ui/theme.md).
- **Android UI modernization** (Compose M3 chrome/overlay) — DONE 2026-05-30 (#362 / #364 / #365). 3 leaf overlays (Symbol/Layout/Candidate) View→Compose M3 over `KeyboardChromeColors`; keys stay custom-draw; `InputView`/window kept View (IME-dismiss bug zone). Memory `project_android_compose_modernization.md`.
- **v3.5.9 D = TPS 三索引** — SHIPPED, tagged `3c8bec16` 2026-05-29. `tps:` FST family parallel to `tl:` / `poj:`; mode-axis (Input + Key + FST) now three-layer symmetric. Retired `is_tps` short-circuit (`dispatch.rs`/`continuous.rs`), `tps_or_mapped_to_er` runtime branch (`search.rs`), `tps_to_tl` canonicalize chain (`classification.rs`). 6 PR (#334-#340, C-0/C-1/C-3a/C-3b/C-4/C-5).
- Roadmap Item 1 (Project Structure & File Naming Cleanup) — CLOSED 2026-05-06 (#212-#215).
- Roadmap Item 4 (Android UI Compose migration) — CLOSED 2026-05-08 (#227-#231).
- v3.5.8 連續輸入 — SHIPPED 2026-05-20 (`61df3028`). See [`docs/releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md) for full plan + Phase status + design rationale + dogfood matrix.

<!-- New active items go in Active / In-flight items. New deferred items go in Out of scope / deferred. Shipped versions get a row in Released versions index + an entry in Closed phases. -->
