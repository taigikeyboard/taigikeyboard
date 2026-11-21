# Annotate in Brackets on every platform — Roadmap

> **Type**: Planning (forward-looking)
> **Keywords**: `annotate in brackets`, `outputBothScripts`, `output_both_scripts`, `commit text`, `tps`, `auto-space`, `desktop settings`, `web`
> **Status**: Accepted 2026-11-21 (maintainer decisions D1–D3 settled). Phase 0 done; Phase 1 next.
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

## Phases

| Phase | Change | Platforms | Status |
|---|---|---|---|
| 0 | This roadmap + maintainer memory | docs | Done 2026-11-21 |
| 1 | TPS multi-word conversion (bugfix) | engine → all | Pending |
| 2 | Bracket units always spaced (D2) | engine, iOS, Android, desktop-core | Pending |
| 3 | Desktop key + settings rows | desktop-core, macOS, Windows, Linux | Pending |
| 4 | Web settings row | Web | Pending |

Phase 2 lands before Phase 3: desktop Auto-Space defaults to off (`desktop/crates/taigi-desktop-core/src/settings/keys.rs:111`), so without it every desktop user who turns brackets on would get `台語 (tâi-gí)我 (guá)`.

### Phase 1 — TPS multi-word conversion (grounded in code)

- **Root cause** (reproduced with a probe against `phonetics`): `engine/phonetics/src/api.rs:98-108` `tl_numeric_to_tps` splits on `-` only. A continuous pick's romanization joins words with spaces (`tâi hó-lâng` → `tai5 ho2-lang5`), and `tps::to_zhuyin` takes one syllable (its contract: the caller pre-splits). `tl_display_to_tps("tâi hó-lâng")` = `ㄉㄞˊ ho2 ㄌㄤˊ`; control `tâi-hó-lâng` = `ㄉㄞˊ ㄏㄛˋ ㄌㄤˊ`.
- **Fix**: split with the existing tokenizer (`engine/phonetics/src/tps.rs:397-407` `tl_syllable_tokens`, public as `api::tl_syllables`), not a new pattern; update the comments at `api.rs:94-97` and `tps.rs:579-582`.
- **Callers** (none relies on spaces staying unsplit): `commit_text.rs:110` (bracket and TPS no-Hanji lead), `composing/src/requests.rs:834` `tps_cell_text`, `phonetics/src/requests.rs:51` (`TlDisplayToTps`), desktop-core `engine/phonetics.rs:34`. Check `linux/.../long_horizon_support/corpus.rs:422`, which splits on `-` before calling.
- **Tests**: `api.rs` unit rows (`tâi hó-lâng`, a sentence, the hyphen-only control); a multi-word bracket row in `commit_text.rs` `tps_truth_table`, with the real romanization from `engine/composing/tests/candidate_dump.rs` (the device pick carries `hór`) and `tps_or_maps_to_er` set explicitly.

### Phase 2 — Bracket units always spaced (D2)

- Inside one composition `nailed.rs:76` `has_word_space_boundary` already returns true under brackets. Across commits the engine answers `earns_auto_space` (`commit_text.rs:271-273`) and each platform gates it on the Auto-Space toggle: Android `CandidateClickHandler.kt:248`, `:346-350`; iOS `ActionHandler+Suggestions.swift:30`, `:56`, `:127-132`; desktop `intent_executor.rs:476-497` via `policies::is_gate_active`.
- Change: a bracketed commit's space no longer depends on the toggle. The design (one new verdict on `CommitResolution` vs. widening the platform gates) is settled in this phase's pre-implementation review.

### Phase 3 — Desktop key and settings rows

- desktop-core: `OUTPUT_BOTH_SCRIPTS = "outputBothScripts"` (default `false`) replaces the RETIRED note at `keys.rs:54-56`; the same spelling as iOS. Add it to `key_path.rs:46-57` `key_path_settings()` (macOS pushes only that list) and to `keys.rs:458-474` `GENERAL_KEYS` (Reset).
- Projection: `CandidateDisplayMode::effective_output_both_scripts(stored) = stored && shows_hanji` beside the existing projections (`settings/document.rs:717-733`), the name the mobile twins use (Android `CandidateDisplayMode.kt`, iOS `SharedSettings.swift:926-927`); `engine_settings()` fills `EngineSettings`, `engine/bridge.rs:111-128` sets the wire field; the test at `bridge.rs:253` flips.
- macOS: toggle under Output Script in `GeneralSettingsView.swift:155-164`, disabled under Romanization Only like the khinsiann toggle; `SettingsStore.Keys`, `resetGeneralSettings` (`SettingsStore.swift:644-661`), `EngineSettings.swift:197-211` defaults; remove the `outputBothScripts` tombstone in `RetiredSettingsCleanup.swift:37-42` (it would delete the setting every launch) and its test; `DesktopCoreRuntimeTests.swift:134-150`.
- Windows `windows/crates/taigi-windows-settings/src/winui/pages/general.rs:119-139`, Linux `linux/crates/taigikeyboard-settings/src/pages/general.rs:79-97`: a switch row under Output Script.
- i18n: `outputBothScripts` (`i18n/settings.json:79-84`) scope gains macos / windows / linux; no new key.
- Upgrade: Windows and Linux have no retired-key sweep, so a hand-edited `outputBothScripts: true` left in `settings.json` takes effect on upgrade; accepted, stated in the PR.
- Visible change: with brackets on, same-Hanji dedupe is off (`engine/composing/src/continuous/dedupe.rs:103`), so the desktop TPS window can show duplicate Hanji cells, as mobile already does.
- Tests: desktop-core `tests/intent_executor.rs` (bracket text, auto-space verdict, Romanization Only masks it), the projection test, `document.rs:896-937` expectations.
- Docs: `behavioral-invariants.md:528`, `:1045`; `engine/protos/proto/envelope.proto:87`.

### Phase 4 — Web

- Row beside the swap control: `web/src/index.html:69-70`, `web/src/settings.js:191-218` (disabled under Romanization Only), `web/src/dev.html:54`. i18n scope gains web on the existing key.
- Playwright: typing `tai5gi2` with brackets on writes `台語 (tâi-gí)`.

## Best practices alignment

| Choice | Reference | Note |
|---|---|---|
| Bracketed second script | none | No IME in `../references/mainstream-ime-comparison.md` writes a bracketed second script |
| Fix in the converter, not a caller-side re-split | `AGENTS.md` § Design principles (no redundant fallback) | One tokenizer for every caller |
| Same key, same projection as mobile | `../contributing/cross-platform-alignment.md` | Intended behavior aligned, per-platform UI |

## Deliberately not adopted

- Per-word brackets inside one whole-sentence pick (D1): the commit would need word boundaries carried through, and the text gets busier.
- Two-line ruby output (#849, closed as not planned).
- A ChromeOS options-page row: that page carries no display setting at all (no swap, no Candidate Display).
