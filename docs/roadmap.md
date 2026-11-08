# TaigiKeyboard — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `roadmap`, `planning`, `open candidates`
> **Status**: Active

---

## Summary

- **Forward-looking work only.** Shipped versions: [`CHANGELOG.md`](../CHANGELOG.md) (release notes per train); the v3.5.8 plan archive is [`releases/v3.5.8/plan.md`](releases/v3.5.8/plan.md). Shipped designs live in the `architecture/*-roadmap.md` docs and in `architecture/behavioral-invariants.md`.
- Release scope, timing and tags are the maintainer's call; only the v3.9.0 fix list below is assigned to a release (maintainer, 2026-10-30).

---

## Active / In-flight items

- **Maintainability audit follow-up (2026-10)** — [`architecture/maintainability-roadmap-2026-10.md`](architecture/maintainability-roadmap-2026-10.md), rounds over [`reports/2026-10-09-audit-all.md`](reports/2026-10-09-audit-all.md). **Paused** by the maintainer 2026-10-14; resume point is the maintainer's call.
- **Web version** — [`architecture/web-roadmap.md`](architecture/web-roadmap.md): the Rust engine in WASM behind a browser shell. Accepted 2026-11-06 (decisions D1–D6 settled). Phases 0–7 done; live at `taigikeyboard.tw/web/`. Phase 4's iPad and AltGr device checks stay open.
- **Windows on Arm64** — [`architecture/windows-arm64-roadmap.md`](architecture/windows-arm64-roadmap.md) (issue #721): Arm64X forwarder + native Arm64 service and settings exe in one universal installer. Done 2026-11-08: Phases 0–4 merged (TSF + forwarder #760, release tooling + CI smoke #764, universal installer #768, aarch64 `check-box.sh` step #777); issue closed.
- **ChromeOS** — [`architecture/chromeos-roadmap.md`](architecture/chromeos-roadmap.md) (issue #759): an MV3 `chrome.input.ime` extension over the web version's wasm engine. Accepted 2026-11-07 (decisions D1–D5 settled). Phase 1 gate passed: 台語 typed through the extension in `linux-chromeos` on a Rosetta VM on the Mac; the MV3 worker stops after ~30 s idle and a key does not wake it, solved by an offscreen-document keepalive. Phase 2 (extension shell) merged #765; Phase 3 (settings) merged #766; Phase 4 (learning) merged #767. Release doc `architecture/chromeos-release.md`; publishing and the real-ChromeOS check wait for the maintainer.

### v3.9.0 fix list (Discord #issues, maintainer 2026-10-30)

Work order = row order. One round per row; a bug row starts with a repro (`AGENTS.md` Core Principle #4). Rows 8–10 need a maintainer design decision first; rows 11–12 need reporter input or a maintainer call.

| # | Item | Kind | Platforms | Next step |
|---|---|---|---|---|
| 1 | Candidates sometimes missing from the candidate bar again; tapping + twice restores them | bug (regression) | Android | Closed 2026-10-30: not reproduced on mobile v3.8.0 (120 device runs, 8 scenarios); no matching earlier fix in git |
| 2 | Continuous input: a shorter-syllable split ranks above the single word (`uê` → ū ê before 鞋; `khikhi`; `guhi`) — TL and TPS, POJ `ôe` fine | bug | engine | Root cause: unigram walker cost (`有`+`的` 8.669 < `鞋` 8.726). Fixed: penalty-only walker bigram, [`architecture/bigram-lm-roadmap.md`](architecture/bigram-lm-roadmap.md) D6, P6.1–P6.3 merged (#563, #564, #565); device dogfood `ue5` / `khikhi` (TL, POJ) / TPS `ㄨㆤˊ` pending. `guhi` not reproduced |
| 3 | Google Docs in Chrome loses candidates on `ko͘` + `c`; same input fine elsewhere | bug | Windows | Root cause: Chrome `GetTextExt` → `TS_E_NOLAYOUT` on every probe once U+0358 sits mid-composition (Docs never reports composition bounds), and the list was dropped. Fixed #569: a window already up keeps its anchor with the new list. Verified on the Windows box in Docs |
| 4 | Show a notice when toggling Hanji / Romanization | feature | macOS, Windows, Linux | Done #570: under side by side the swap announces the output script on every desktop, candidate list open or not (macOS / Windows flash, Fcitx5 notice; IBus symbol only). Combined / Romanization Only stay silent |
| 5 | Custom layouts: allow ⁿ on the main keyboard | feature | iOS, Android | Done #571: ⁿ (U+207F) offered in the spare tray after o͘ as a plain key; Shift gives ᴺ |
| 6 | Vertical expandable candidate window layout | feature | Windows, macOS, Web | Done #572 (core + Windows + Linux→Vertical + i18n), #573 (macOS twin; Expandable label → Horizontal Expandable / 水平展開). Reporter follow-up, USER 2026-11-08 approved (reverses the 2026-10-31 `←`-folds-anywhere call): unfolds into 9-row columns, 3 a page (Microsoft Bopomofo shape) — keys 1–9 number the keyed column, `←` / `→` step a column, `←` folds only from the first column, PageUp / PageDown + chevron page; PR1 core + Windows + Web #778, PR2 macOS twin #780 — done |
| 7 | Custom layouts: user-editable long-press callouts (half-width symbols on half-width `,` `.`; `{ }` on `( )`) | feature | iOS, Android | Done #577, #579: long-press lists editable per key. #691 adds the second symbol page to the board (`{` `«` `€` `→`; `{` types `｛` under full-width punctuation) and dims tray characters a selected key cannot take. Dogfood S133 |
| 8 | Custom layouts are not separated by TL / POJ, so picking one when switching romanization is hard | feature | iOS, Android | Done #666: Tâi-lô and Pe̍h-ōe-jī each remember their own layout (built-in or custom), switching scripts brings it back; Save picks where a custom layout shows (Universal / Tâi-lô / Pe̍h-ōe-jī dropdown). Dogfood S130 |
| 9 | `c` types `ch` / `ts` (toggle); desktop Hanji → romanization reverse lookup | feature | desktop | Decision: toggle vs. losing a selection key; reverse lookup scope |
| 10 | TPS requests (6 parts) | feature | desktop, mobile | Split 2026-11-08, maintainer approved the recommended arms — see § Row 10 below |
| 11 | Neutral-tone candidates for 起來 / 起去 / 一下 / ah | feature | engine | Needs input → expected-output examples |
| 12 | Dialectal D spellings for L / J readings | feature | engine | Maintainer call; phonetics rules need an authoritative source (`docs/contributing/phonetics.md`) |

#### Row 10 — TPS requests (Discord 1561728886658830456, maintainer 2026-11-08 「依據使用者建議+你的建議處理」)

One PR per part, in this order. The mobile layout (reporter's second image) is a separate mobile round, not scheduled here.

| Part | Today (`file:line`) | Change | Status |
|---|---|---|---|
| 10a Window off = glyph mode | `desktop-core/src/engine/bridge.rs` asks for the Hanji conversion under TPS whatever Show Candidate Window says; with the window off ↓ commits Hanji (`keys/intent.rs` `tps_window_key`) | No conversion while the window is off, so Enter writes the glyphs — as TL / POJ write the romanization with the window off (S33) | Merged #804 2026-11-09 |
| 10b Tone-8 dot | Keyboards type U+02D9 (`keys/tps_layout.rs` `7`, mobile too); the converter writes U+0307 over a stop coda (`engine/phonetics/src/tps.rs` `ZHUYIN_TONES`) | The engine stores U+0307 for a tone-8 dot typed right after ㆴ ㆵ ㆻ ㆷ (all platforms) | Merged #805 2026-11-09 |
| 10c Desktop layout | `keys/tps_layout.rs` `KEYS` | Reporter's first image: `m` ㆨ, `.` / `>` ㆦ / ㆧ, `/` / `?` ㄥ / ㆭ, `-` / `_` ㆲ / ㆱ, Shift+`p` ㆬ, stop codas `t` ㆴ `g` ㆵ `b` ㆻ; the hyphen moves to `=`. Dachen-shared keys unchanged | Merged #806 2026-11-09 |
| 10d ↓ lists the word before the caret | After a pick, a caret at the tail's start lists the next word (H-P5 accepted boundary, `engine/composing/src/conversion.rs` `anchor`) | ↓ at the tail's start lists the last nailed word; opening the list changes nothing, a pick re-opens that word and nails again (deferred re-open, pre-impl review 2026-11-08) — the list is always the word before the caret | Merged #807 2026-11-09 |
| 10e Custom words in the walk | The TPS walk passes no custom rows (`conversion.rs` `convert`, `custom: &[]`), so 陳三 shows as 陳相 and lists only from 陳 | Custom-dictionary words become walk edges, as they rank in TL / POJ | Merged #808 2026-11-09 |

---

## Open candidates (unscheduled)

One line each; the linked section holds the design, the measurements and the open questions.

**Engine and dictionary**

| Item | Status | Where |
|---|---|---|
| Retire the old dictionary `frequency` after E1 | Recorded, not started. Still read: the equal-cost tie-break (`CandidateSortKey` −score / −freq; ~69 % of rows are corpus-unseen and share one cost), the search-page tie-break, `association.bin` dictionary-derived counts (`associations.py`), CSV / FST row order + duplicate max-aggregation (`merge_csv.py`), the `dict.bin` u32 field, `CandidateMessage.score`. Steps: pick a replacement tie-break for unseen words, a count source for the association rows, then the format bump. A custom twin of a dictionary word carries no corpus cost (sorts below its priced homophone in the list) | [`reports/2026-10-08-e1-p5b-list-sort.md`](reports/2026-10-08-e1-p5b-list-sort.md) §5 |

**Mobile parity (inventory 2026-10-12)** — pending code; directions decided unless marked.

| Item | Direction | Where |
|---|---|---|
| Needs device repro first (UNCONFIRMED) | Android symbols-page `、` / numeric pad wipes the preedit; Android next-word predictions survive a cursor move | `+Cleanup.swift`, `TextInputManager.kt`, `TextInputKeyHandler.kt` |

## Not to re-propose

Closed by the maintainer; one line each.

- `zh-TW` README; xcconfig signing; Android Gradle proto plugin; dictionary-source licensing follow-up.
- Romanization spelling correction (fuzzy / edit-distance tolerance in the engine) — dropped when the Candidate Display = Romanization mode shipped without it.
- Corpus expansion P1b, including the TAT application.
- A full TL / POJ / en / ja proofreading pass of the app UI — reported typos are fixed per incident.
- `$` sentence-start opener (its `association.bin` data is removed in its own PR).
- Smart-suggestion techniques outside the bigram model: 3 zero-query continuation from learned history, 5 negative feedback on rarely picked entries, 6 zero-query rule table (`「`→`」`, number → measure word), 9 word → emoji, 10 offensive-word filter, 11 privacy gates (incl. the Android `IME_FLAG_NO_PERSONALIZED_LEARNING` gap). Names of all eleven: [`architecture/bigram-lm-roadmap.md`](architecture/bigram-lm-roadmap.md).
- Windows candidate-window paint latency (frame drawn before its candidates on a busy host UI thread) and UIA exposure.
- Converted-romanization commit on Enter (`suann2ting3` → `suán-tìng`).
- TL mode (臺羅模式) tone key commits the syllable without a candidate window (MOE Mac IME parity).
- Mobile predictions after Space.
- Bigram P7 hanji-only sources, D7 two-word context (P6 walker term shipped 2026-10-30).
- e2e drivers for macOS, Windows, Android, iOS.
- Invariant-label PR2; naming batch C (persisted names stay frozen).
- Shared Swift package for macOS + iOS — see [`architecture/maintainability-roadmap-2026-10.md`](architecture/maintainability-roadmap-2026-10.md) § Best practices alignment (deliberately not adopted).
