# Annotate in Brackets on every platform — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `annotate in brackets`, `outputBothScripts`, `output_both_scripts`, `commit text`, `tps`, `auto-space`, `desktop settings`, `web`
> **Status**: Accepted 2026-11-21 (maintainer decisions D1–D4 settled). Phases 0–3 done; Phase 4 next.
> **Siblings**: `behavioral-invariants.md` (§ Annotate in Brackets rows), `../engine/continuous-commit-and-display.md` §10.2.

---

## Summary

- **Goal**: Annotate in Brackets (`台語 (tâi-gí)` / `tâi-gí (台語)`), today mobile-only, works the same on macOS, Windows, Linux and Web, after two engine defects found on a device are fixed.
- **Why**: the two-line (ruby) request #849 was closed as not planned; brackets are the alternative offered to desktop users.
- The engine already writes the bracketed text (`engine/composing/src/commit_text.rs` `resolve_commit_text` / `lead`); desktop-core retired its key on 2026-09-30 (`1b0e64cdf`), so the desktops and Web always send `output_both_scripts = false`.

## Device findings (Android, 2026-11-21)

Android debug 3.10.0 built from `main` on 2026-11-21, Samsung Notes, Annotate in Brackets on. Text read from the host field.

| Case | Output | Verdict |
|---|---|---|
| Whole sentence in one pick (TL, Hanji first) | `我欲去學校 (Guá beh khì ha̍k-hāu)` | Correct — one bracket per pick (D1) |
| Same sentence in three picks | `我 (Guá) 欲 (beh) 去學校 (khì ha̍k-hāu)` | Correct — one bracket per pick (D1) |
| Romanization first, POJ, khinsiann `tsáu--khì`, hyphen `ti-á`, §34 literal `Goo-lú-huh` (no bracket), `，` replacing the auto space, next-word pick | as expected | Correct |
| Word-by-word picks with Auto-Space **off** | `台語 (Tâi-gí)，我 (guá)講 (kóng)。` | Defect — no space between bracket units across commits, while inside one composition `engine/composing/src/nailed.rs:76` always spaces them (Phase 2) |
| TPS multi-word pick `ㄉㄞˊㄏㄜˋㄌㄤˊ` → 台好人 | `台好人 (ㄉㄞˊ hor2 ㄌㄤˊ)` | Defect — Bopomofo conversion stops at the first syllable of each space-joined word (Phase 1) |

## Decisions (maintainer, 2026-11-21)

| # | Question | Decision |
|---|---|---|
| D1 | One bracket per pick, or per word inside a whole-sentence pick? | One bracket per pick; no engine change |
| D2 | With Auto-Space off, do bracket units still get a space between them? | Yes: with brackets on, a space always separates two bracket units, whatever Auto-Space says; punctuation still replaces it |
| D3 | Fix the TPS defect on its own first? | Yes, its own bugfix PR |
| D4 | Brackets under desktop TPS (its Hanji conversion commits as shown: only picked words bracketed, no space after)? | Off under TPS, like Show Typed Text First |

## Phases

| Phase | Change | Platforms | Status |
|---|---|---|---|
| 0 | This roadmap + maintainer memory | docs | Done 2026-11-21 |
| 1 | TPS multi-word conversion (bugfix) | engine → all | Merged #1058 `bc70d4872`; Android device verified |
| 2 | Bracket units always spaced (D2) | iOS, Android | Merged #1059 `ec0350de4`; Android device verified |
| 3 | Desktop key + settings rows + the D2 spacing gate | desktop-core, macOS, Windows, Linux | Merged #1060 `40e90a77e`; device check pending |
| 4 | Web settings row | Web, ChromeOS, browser extension | In review |

The desktop D2 gate ships with the desktop key in Phase 3, not before: desktop Auto-Space defaults to off (`desktop/crates/taigi-desktop-core/src/settings/keys.rs:111`), so the key without the gate would give every desktop user who turns brackets on `台語 (tâi-gí)我 (guá)`, and the gate without the key would be dead code.

### Phase 1 — TPS multi-word conversion (grounded in code)

- **Root cause** (reproduced with a probe against `phonetics`): `engine/phonetics/src/api.rs:98-108` `tl_numeric_to_tps` splits on `-` only. A continuous pick's romanization joins words with spaces (`tâi hó-lâng` → `tai5 ho2-lang5`), and `tps::to_zhuyin` takes one syllable (its contract: the caller pre-splits). `tl_display_to_tps("tâi hó-lâng")` = `ㄉㄞˊ ho2 ㄌㄤˊ`; control `tâi-hó-lâng` = `ㄉㄞˊ ㄏㄛˋ ㄌㄤˊ`.
- **Fix**: split with the existing tokenizer (`engine/phonetics/src/tps.rs:397-407` `tl_syllable_tokens`, public as `api::tl_syllables`), not a new pattern; update the comment at `api.rs:94-97` (`tps.rs`'s "caller pre-splits on `[-\s]+`" is already true).
- **Callers** (none relies on spaces staying unsplit): `commit_text.rs:110` (bracket and TPS no-Hanji lead), `composing/src/requests.rs:834` `tps_cell_text`, `phonetics/src/requests.rs:51` (`TlDisplayToTps`), desktop-core `engine/phonetics.rs:34`. Check `linux/.../long_horizon_support/corpus.rs:422`, which splits on `-` before calling.
- **Tests**: `engine/phonetics/tests/op_coverage.rs` rows (`tâi hó-lâng`, a three-word sentence, the hyphen-only control); a multi-word bracket row in `commit_text.rs` `tps_truth_table`, and the device reading `tâi hór-lâng` under both `tps_or_maps_to_er` values in `tps_bopomofo_follows_the_or_setting`.

### Phase 2 — Bracket units always spaced (D2) (grounded in code)

- Inside one composition `nailed.rs:76` `has_word_space_boundary` already returns true under brackets. Across commits the engine answers `earns_auto_space` (`commit_text.rs:271-273`) and the phones AND it with the Auto-Space toggle: Android `CandidateClickHandler.kt:165` (prediction), `:216` (Continuous); iOS `ActionHandler+Suggestions.swift:30`, `:56`; the swap re-reads the toggle (Android `TaigiKeyboard.kt` `isAutoSpaceSwapArmed`, iOS `ActionHandler.swift`).
- Design (pre-implementation review 2026-11-21, Fable, Codex skipped): a platform predicate `spacesCandidateCommits(isAutoSpaceEnabled, isOutputBothScripts)` = either setting (the effective brackets value the phones already send the engine), read at those four commit sites and by the swap. No engine or proto change: a `CommitResolution` field was rejected because the swap needs the platform-side value anyway.
- Scope (maintainer 2026-11-21): every candidate or prediction commit that earns a space, a Hanji-less one included (`gua 講 (kóng)`); the raw-preedit and English-word commits (`appendAutoSpaceIfEarned`) keep plain Auto-Space (`gua講 (kóng)` after Enter).
- Pins: `behavioral-invariants.md` §23 `INVARIANT_BRACKETS_SPACE_CANDIDATE_COMMITS`.

### Phase 3 — Desktop key, settings rows and the D2 spacing gate

- desktop-core: `OUTPUT_BOTH_SCRIPTS = "outputBothScripts"` (default `false`) replaces the RETIRED note at `keys.rs:54-56`; the same spelling as iOS. Add it to `key_path.rs:46-57` `key_path_settings()` (macOS pushes only that list) and to `keys.rs:458-474` `GENERAL_KEYS` (Reset).
- Projection: `SettingsDocument::effective_output_both_scripts()` = stored && `allows_output_both_scripts()` (shows Hanji, not TPS — D4) beside the existing projections (`settings/document.rs:717-733`), the name the mobile twins use (Android `CandidateDisplayMode.kt`, iOS `SharedSettings.swift:926-927`); `engine_settings()` fills `EngineSettings`, `engine/bridge.rs:111-128` sets the wire field; the test at `bridge.rs:253` flips.
- macOS: toggle under Output Script in `GeneralSettingsView.swift:155-164`, disabled under Romanization Only and TPS (D4; like the C-affricate row under TPS); `SettingsStore.Keys`, `resetGeneralSettings` (`SettingsStore.swift:644-661`), `EngineSettings.swift:197-211` defaults; remove the `outputBothScripts` tombstone in `RetiredSettingsCleanup.swift:37-42` (it would delete the setting every launch) and its test; `DesktopCoreRuntimeTests.swift:134-150`.
- Windows `windows/crates/taigi-windows-settings/src/winui/pages/general.rs:119-139`, Linux `linux/crates/taigikeyboard-settings/src/pages/general.rs:79-97`: a switch row under Output Script, greyed by desktop-core `SettingsDocument::allows_output_both_scripts`.
- i18n: `outputBothScripts` (`i18n/settings.json:79-84`) scope gains macos / windows / linux; no new key.
- Upgrade: Windows and Linux have no retired-key sweep, so a hand-edited `outputBothScripts: true` left in `settings.json` takes effect on upgrade; accepted, stated in the PR.
- D2 gate (Phase 2 rule): `composing/intent_executor.rs` `append_auto_space` / `auto_space_gate` serve both a candidate commit and `CommitRaw` / `CommitAsShown` (`finalized_composition_resolution`), so the gate must tell them apart — only the candidate commit takes `auto || effective brackets`; the `auto_space_enabled` passed to `confirm_composition_then_insert` (`:157`) and the swap sites (`pass_through_may_consume`, `pass_through_may_swap`, `swap_auto_space`) follow the same rule. Settled in the pre-implementation review (2026-11-21, Fable, Codex skipped): a `ConfirmCompositionThenInsert.candidate_spacing_enabled` field, read by the engine only when the pick finalizes the composition (`transition.rs` `confirm_composition_then_insert`); a `CommitResolution` field comes too late, since the engine writes the space inside the transition. macOS `ComposingBackend.swift` `panel(_:attaches:)` dropped its own Auto-Space check; the core decides.
- Visible change: under TL / POJ with brackets on, same-Hanji dedupe is off (`engine/composing/src/continuous/dedupe.rs:103`), so two readings of one Hanji word show as two cells, as on mobile (they write different strings). Under TPS nothing changes (D4).
- Tests: desktop-core `tests/intent_executor.rs` (bracket text, auto-space verdict, Romanization Only masks it), the projection test, `document.rs:896-937` expectations.
- Docs: `behavioral-invariants.md:528`, `:1045`; `engine/protos/proto/envelope.proto:87`, `composing.proto:343` (the platform no longer ANDs plain Auto-Space).

### Phase 4 — Web

- Row beside the swap control: `web/src/index.html:69-71`, `web/src/settings.js:220-222` (disabled under Romanization Only and TPS, `allows_output_both_scripts`), `web/src/dev.html:59-61` with the same rule in `web/src/dev.js` `showControls`. i18n scope gains web on the existing key.
- Playwright: typing `tai5gi2` with brackets on writes `台語 (tâi-gí)`.
- The ChromeOS and browser-extension options pages are cut from the same form (`web/chromeos/options-form.mjs`), which hides only rows those packages cannot honour; Output Script and Candidate Display show there, so the row does too, over the same desktop-core session (maintainer 2026-11-21).

## Best practices alignment

| Choice | Reference | Note |
|---|---|---|
| Bracketed second script | none | No IME in `../references/mainstream-ime-comparison.md` writes a bracketed second script |
| Fix in the converter, not a caller-side re-split | `AGENTS.md` § Design principles (no redundant fallback) | One tokenizer for every caller |
| Same key, same projection as mobile | `../contributing/cross-platform-alignment.md` | Intended behavior aligned, per-platform UI |

## Deliberately not adopted

- Per-word brackets inside one whole-sentence pick (D1): the commit would need word boundaries carried through, and the text gets busier.
- Two-line ruby output (#849, closed as not planned).
