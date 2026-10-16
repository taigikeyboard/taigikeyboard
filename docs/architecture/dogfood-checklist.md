# Dogfood Checklist (real-device acceptance gate)

> **Type**: Reference (living)
> **Keywords**: `dogfood`, `acceptance`, `perf-gate`, `invariant`
> **Related**: behavioral-invariants.md, ../contributing/known-pitfalls.md

The qualitative perf gate (`docs/contributing/known-pitfalls.md` § Review and device gates) made concrete. Read this **when preparing a device dogfood pass or when a PR / memory entry cites an `Sn` item** — it is not always-on context.

- Base checklist: **S1 POJ diacritics**, **S2 TPS composition**, **S3 Hanji candidate scroll**, plus iOS keyboard-extension 64 MB hard cap, leak-free + no-keyboard-dismiss. Translates "perceptible regression on real interactive sequences" into a concrete acceptance gate.

A pending **Sn** = a dogfood acceptance item written in full: **type X → expect Y** + the `INVARIANT_*` it pins. Full root-cause / fix-file receipts live in `docs/architecture/behavioral-invariants.md §N` — not duplicated here. Recurring patterns (state once):

- **Evasion shape** (S4/S5/S9/S11/S13/S15/S17–S21): every hermetic fixture used a single reading / family / initial-form, so the production collision never fired in tests → real-device + the `engine/composing/tests/candidate_dump.rs` dev harness (production artifacts) are the catch-net. Android JVM can't load the `.so`/SQLite, so SQL-path items are dogfood-pinned there.
- **Fix-location lesson** (S5/§18, reused by S9/S17/S22): a display-only candidate-strip change belongs in the span-local key builder (`composing::shadow` / display-layer seam), never the shared segmentation primitive (`syllabifier::valid_span_endings`) — touching the primitive reintroduces the #290 non-greedy-recovery regression.
- **Build gate**: engine-touching items need `make build` (refresh xcframework/jniLibs) before device dogfood; `make dict` only when dict artifacts change. Platform-only items skip both.

Index (S7, S88, S109 absent). Every one-line item below passed device dogfood (USER 2026-10-02 and 2026-10-13: "all dogfood passed"); an item written in full is pending. The full "type X → expect Y" text of each item lives in git history (`git log -p -- docs/architecture/dogfood-checklist.md`); root cause and fix live in `behavioral-invariants.md §N`. A new item is added in full while pending and collapsed to one index line once it passes.

- **S4** explicit-tone candidate filter — 2026-05-30 · §17 · #367
- **S5** longest-match prefix suppression — 2026-05-31 · §18 · #371
- **S6** first-candidate keycap-color hint — 2026-06-01 · §19 · #373
- **S8** leading khinsiann `--` literal — 2026-06-02 · §21 · #379
- **S9** continuous slot-0 respects dict separator — 2026-06-02 · §22 · #380
- **S10** auto-space attaching-punctuation swap — 2026-06-02 · §23 · #381
- **S11** association cross-mode recall + no duplicate — 2026-06-03 · §24 · #382
- **S12** Continuous-input commit carries canonical TL (write-layer root fix) — 2026-06-03 · §25
- **S13** Custom words findable across input modes — 2026-06-03 · §26 · #384
- **S14** Custom-word row-count cap parity — 2026-06-03 · §27
- **S15** Polyphonic-character frequency (Hanji,canonical-TL) pair-key — 2026-06-04 · §28 · #386
- **S16** user data excluded from OS auto-backup — 2026-06-04 · §29 · #388
- **S17** Literal composing — TL+POJ, no spelling conversion — 2026-06-05 · §30 · #390
- **S18** TPS space key = soft syllable separator (tone-1 continuous input) — 2026-06-05 · §31 · #391
- **S19** TPS stop-coda gate — tone 1 without space — 2026-06-05 · §32 · #392
- **S20** TPS nasal-coda gate — tone 1 without space — 2026-06-06 · §33 · #394
- **S21** TPS de-fold enumerate — hidden words surface as candidates — 2026-06-06 · §35 · #396
- **S22** Literal romanization candidate — fast Hanji-with-romanization input — 2026-06-06 · §34 · #395 · #397
- **S23** TPS ambiguity-aware lookup — one-key-many-glyph ambiguity is deferred to the dictionary — 2026-08-19 · §35
- **S24** TPS space pins the unmarked tones (tone 1 / tone 4) — 2026-08-20 · §41
- **S25** A candidate may not have more syllables than the input — 2026-08-21 · §43
- **S26** Candidate display = Romanization Only — 2026-09-01 · §44 · §42 · #7
- **S27** Candidate display = Hanji with Romanization (Hanji and roman mixed) — 2026-09-01 · §42 · #666
- **S28** Nasal rime `o͘ⁿ` alias spellings + double-tap nn/oo — 2026-09-04 · §45 · #685 · #686
- **S29** Windows app name follows the system locale — 2026-09-05
- **S30** macOS font-management pane + custom font upload — 2026-09-08
- **S31** Windows font-management pane + custom font upload — 2026-09-08
- **S32** Telex tone keys — 2026-09-09 · #17 · #19
- **S33** Candidate-window toggle — 2026-09-09 · #20
- **S34** Telex guide shortcut — 2026-09-09 · #21 · #22
- **S35** The literal roman cell has no slot key — 2026-09-09 · §34
- **S36** Symbol picker — 2026-09-09 · #26
- **S37** Composing caret moves left / right — 2026-09-09
- **S38** ⇧ + slot key = pick a cell in the other script for this commit — 2026-09-10
- **S39** Custom font delete-then-re-add + row name = filename — 2026-09-10 · #37
- **S40** Adding a font no longer crashes; list selection stays put — 2026-09-11
- **S41** Typing with a custom font no longer kills the host — 2026-09-11
- **S42** Font-management lists installed typefaces (macOS) — 2026-09-11
- **S43** Font-management lists installed typefaces (Windows) — 2026-09-11
- **S44** NextWord prediction row follows Candidate Display + the literal cell inherits the dictionary identity — 2026-09-12
- **S45** First key after switching is no longer lost (Android) — 2026-09-12 · §13
- **S46** Partial-tone input keeps the typed tone (all four platforms) — 2026-09-14 · §17
- **S50** Abbreviation input reaches the abbreviated words (all four platforms) — 2026-09-18 · §46
- **S51** Symbol picker keys stay in the picker under Chromium hosts (macOS) — 2026-09-18 · §47
- **S52** Hanji-first out of the box, and the desktop General pane owns its output script and reset (all four platforms) — 2026-09-18 · §48
- **S53** Symbol picker: `⋯` and the recent picks lead (macOS + Windows) — 2026-09-19
- **S54** Custom theme: one background surface, gradient direction, scheme-invariant colours, new editor order (iOS PR A; Android PR B) — 2026-09-19
- **S57** Telex `x` = tone 1 / 4, `v` = tone 2 / 8 (macOS + Windows) — 2026-09-19
- **S55** Custom theme: photo background (iOS PR C; Android PR D) — 2026-09-20
- **S58** Shortcut recorder takes Enter on any composing row, and bare Tab (macOS + Windows) — 2026-09-19
- **S59** Input-source menu lists the global shortcuts (macOS + Windows) — 2026-09-19
- **S56** Closed tone-8 syllable no longer trails its shorter prefix family (all four platforms) — 2026-09-19 · §18
- **S60** No Hyphens: candidates and commits drop the dictionary hyphen, khinsiann `--` becomes `·` (all four platforms) — 2026-09-20 · §49
- **S61** About page from the input-source menu (macOS + Windows) — 2026-09-20
- **S62** Learned phrases — a phrase composed segment by segment becomes one candidate (all four platforms) — 2026-09-20 · §34 · #109
- **S63** Ctrl + punctuation types the other width once (macOS + Windows) — 2026-09-20 · #119 · removed 2026-10-07 (S111)
- **S64** Shortcuts pane shows the three fixed rows: Pick a Candidate Directly / Move Through Candidates / Delete What You Are Typing (macOS + Windows) — 2026-09-20
- **S65** Settings rows in pipeline order (macOS + Windows) — 2026-09-21
- **S66** Typed separator joins the continuous best reading (all four platforms) — 2026-09-21 · #129
- **S67** Frequency Records / Association Records pages removed, recording always on (iOS + Android) — 2026-09-22 · #130
- **S68** Typed separator: committed between picks, remembered by the learned phrase (all four platforms) — 2026-09-22 · #131
- **S69** Typed hyphen is a syllable boundary (all four platforms) — 2026-09-22 · #5655
- **S70** "ⁿ becomes ᴺ in capitals" toggle (all four platforms) — 2026-09-22 · §53
- **S71** Typing after a nail keeps the Hanji-first prefix unspaced (iOS + Android) — 2026-09-22 · §54 · #103
- **S72** Caps Lock keeps a POJ custom entry all caps (all four platforms) — 2026-09-22 · #89
- **S73** Typed hyphen renders inside a dictionary word (all four platforms) — 2026-09-22 · §55 · #5517
- **S74** Linux first machine — Fcitx5 on KDE Plasma, then IBus on GNOME (Ubuntu 24.04 VM) — 2026-09-23
- **S75** One size pop-up for the candidate window, Standard a step smaller (macOS + Windows) — 2026-09-23
- **S76** Linux bundled typefaces are system fonts; no Manage Fonts pane (Ubuntu VM, both shells) — 2026-09-24
- **S77** Identical desktop menus; no update check on Linux (Ubuntu VM, KDE + Fcitx5 then GNOME + IBus; macOS; Windows) — 2026-09-25 · #193
- **S78** Custom theme: one Key Fill row, white by default (iOS + Android) — 2026-09-26
- **S79** Custom theme card: background only, no sample key (iOS + Android) — 2026-09-26
- **S80** Custom theme photo: decoded off the main thread (iOS + Android) — 2026-09-26
- **S81** One-handed mode from the toolbar keyboard button (iOS + Android) — 2026-09-27
- **S82** About page: Facebook + Instagram + Threads rows (all five platforms) — 2026-09-27
- **S83** Next-word strip predicts from the committed word (iOS + Android) — bigram LM P4a
- **S84** Composing candidates re-ranked by the previous word (iOS + Android) — bigram LM P5
- **S85** A continuous composition teaches its word pairs (iOS + Android) — bigram LM P4b
- **S86** Android: a selected word ending in sentence punctuation is still learned — 2026-09-30 · §40
- **S87** Next-word: a clause mark breaks the context — 2026-10-13 · §40
- **S89** Windows + Linux: Switch TPS and the romanization it returns to — 2026-10-13
- **S90** macOS: TPS typing and Switch TPS — 2026-10-13
- **S91** macOS + Windows + Linux: the TPS candidate window on demand — 2026-10-13
- **S92** macOS + Windows + Linux: the TPS key panel, show-only — 2026-10-13
- **S93** macOS + Windows: clicking the TPS key panel — 2026-10-13
- **S94** macOS + Windows + Linux: Hanji conversion in the TPS preedit — 2026-10-13
- **S95** macOS: the TPS candidate window sits under the word it lists — 2026-10-13 · §59
- **S96** all five platforms: Nasal mark in POJ capitals is a picker — 2026-10-13 · §53
- **S97** iOS + Android: `·` (U+00B7) from a long-press on `-` — 2026-10-13
- **S98** macOS + Windows + Linux: the caret chord ↑ / ↓ and Home / End jump to the start / end — 2026-10-13
- **S99** Windows + Linux + macOS: the input indicator names the script and what a commit writes — 2026-10-13 · §60
- **S100** iOS + Android: Word Frequency and Phrases sit under one Learning Records row — 2026-10-13
- **S101** all five platforms: Add to Custom Dictionary moves a learned phrase — 2026-10-13
- **S102** iOS + Android: dictionary-source toggles move to a Manage Dictionaries page — 2026-10-13
- **S103** all five platforms: the chosen separator form of a word leads — 2026-10-13 · §22
- **S104** iOS + Android: the dictionary search bar moves to the Manage Dictionaries page — 2026-10-13
- **S105** all five platforms: Add to Custom Dictionary copies a word-frequency row — 2026-10-13
- **S106** all five platforms: Syllable Separator replaces No Hyphens — 2026-10-13 · §49
- **S107** all five platforms: Delete Learning Records moves to the Learning Records page — 2026-10-13
- **S108** iOS + Android: Settings tab regrouped, one row style — 2026-10-13
- **S110** macOS + Windows + Linux: TPS types only full-width punctuation — 2026-10-13
- **S111** macOS + Windows + Linux: Ctrl + punctuation no longer types the other width — 2026-10-13
- **S112** macOS + Windows: the TPS key panel lights the key just typed — 2026-10-13
- **S113** Linux GNOME + IBus: Taigi keeps a Dvorak / Colemak layout — 2026-10-13
- **S114** Windows: Keyboard Layout = Dvorak / Colemak — 2026-10-13
- **S115** Linux: the literal roman cell has no slot key — 2026-10-13 · §34
- **S116** iOS + Android: next-word after a corpus re-split — 2026-10-13
- **S117** one mobile + one desktop platform: slot 0 on one corpus scale — 2026-10-13
- **S118** one mobile + one desktop platform: slot 0 writes the word the corpus writes most — 2026-10-13
- **S119** one mobile + one desktop platform: the candidate list and the dictionary search page follow the corpus — 2026-10-13
- **S120** one mobile + one desktop platform: a TPS toneless first syllable is listed beside the longer one that straddles the next — 2026-10-13 · §18 guard (e)
- **S121** A TPS tone mark or space pins the reading (Android + Windows) — 2026-10-11 · §17 · #479
- **S122** one mobile + one desktop platform: a custom word waits until its final syllable is typed into — 2026-10-13 · §43
- **S123** Windows: hosts and input outside Notepad (UI-less host, password field, AltGr, RDP, Chrome, Terminal, touch keyboard, high contrast) — 2026-10-13
- **S124** Windows: settings window (WinUI 3) — native text field, Narrator / Tab, dialogs, adaptive width, theme, shortcut recording — 2026-10-13
- **S125** Windows: install, update, uninstall — 2026-10-13
- **S126** macOS + Windows + Linux: two keys per composing action, Space confirms (Discord #issues, a Linux request; defaults USER 2026-10-15) — pending. Settings → Shortcuts: every Candidate Selection and Output row shows two fields; Confirm Key shows Enter and Space, Output the Other Script (Hanji / Romanization) shows "None" in both. Type `tai5`: Enter and Space both commit the current script; ⇧ + a slot key still commits that slot in the other script. Under TPS, Space mid-composition with no list open is still the separator. Record Space on Output the Other Script → Confirm Key's second field empties, and Space writes the other script again (the pre-2026-10-15 behaviour). Clear a field with its ×: only that field empties; the × of an empty field stays in place, disabled. Reset to Defaults → back to the state above. Upgrade: an install that never touched its shortcuts now confirms on Space. Build gate: `make macos-libs` before the macOS build.
- **S127** one mobile + one desktop platform: a custom abbreviation follows the readings — pending. With a custom 退伍 `thui-ńg`, type `thng`: 湯 / 糖 / 褪 and the other single-syllable rows come first, 退伍 after them; `tkt` with a custom 陳建中 `Tân Kiàn-tiong` still lists 陳建中 first. Build gate: `make build` (engine change)
