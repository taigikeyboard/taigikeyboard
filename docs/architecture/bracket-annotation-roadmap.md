# Annotate in Brackets on every platform — Roadmap

> **Type**: Design record (roadmap done)
> **Keywords**: `annotate in brackets`, `outputBothScripts`, `output_both_scripts`, `commit text`, `tps`, `auto-space`, `desktop settings`, `web`
> **Status**: Done — Phases 0–4 merged 2026-11-21 (#1058–#1061); the Phase 3 desktop device check is pending. Decisions D1–D4.
> **Siblings**: `behavioral-invariants.md` (§ Annotate in Brackets rows), `../engine/continuous-commit-and-display.md` §10.2.

---

## Summary

- **Goal**: Annotate in Brackets (`台語 (tâi-gí)` / `tâi-gí (台語)`), once mobile-only, works the same on macOS, Windows, Linux and Web (ChromeOS and the browser extension included), after two engine defects found on a device were fixed.
- **Why**: the two-line (ruby) request #849 was closed as not planned; brackets are the alternative offered to desktop users.
- The engine writes the bracketed text (`engine/composing/src/commit_text.rs` `resolve_commit_text` / `lead`); every platform only stores the setting and sends `output_both_scripts`.

## Device findings (Android, 2026-11-21)

Android debug 3.10.0 built from `main` on 2026-11-21, Samsung Notes, Annotate in Brackets on. Text read from the host field.

| Case | Output | Verdict |
|---|---|---|
| Whole sentence in one pick (TL, Hanji first) | `我欲去學校 (Guá beh khì ha̍k-hāu)` | Correct — one bracket per pick (D1) |
| Same sentence in three picks | `我 (Guá) 欲 (beh) 去學校 (khì ha̍k-hāu)` | Correct — one bracket per pick (D1) |
| Romanization first, POJ, khinsiann `tsáu--khì`, hyphen `ti-á`, §34 literal `Goo-lú-huh` (no bracket), `，` replacing the auto space, next-word pick | as expected | Correct |
| Word-by-word picks with Auto-Space **off** | `台語 (Tâi-gí)，我 (guá)講 (kóng)。` | Defect — no space between bracket units across commits, while inside one composition `nailed.rs` `has_word_space_boundary` always spaces them (Phase 2) |
| TPS multi-word pick `ㄉㄞˊㄏㄜˋㄌㄤˊ` → 台好人 | `台好人 (ㄉㄞˊ hor2 ㄌㄤˊ)` | Defect — Bopomofo conversion stops at the first syllable of each space-joined word (Phase 1) |

## Decisions (maintainer, 2026-11-21)

| # | Question | Decision |
|---|---|---|
| D1 | One bracket per pick, or per word inside a whole-sentence pick? | One bracket per pick; no engine change |
| D2 | With Auto-Space off, do bracket units still get a space between them? | Yes: with brackets on, a space always separates two bracket units, whatever Auto-Space says; punctuation still replaces it |
| D3 | Fix the TPS defect on its own first? | Yes, its own bugfix PR |
| D4 | Brackets under desktop TPS (its Hanji conversion commits as shown: only picked words bracketed, no space after)? | Off under TPS, like Show Typed Text First |

## Phases

| Phase | Change | Platforms |
|---|---|---|
| 1 | TPS multi-word conversion (bugfix, #1058) | engine → all |
| 2 | Bracket units always spaced (D2, #1059) | iOS, Android |
| 3 | Desktop key + settings rows + the D2 spacing gate (#1060) | desktop-core, macOS, Windows, Linux |
| 4 | Web settings row (#1061) | Web, ChromeOS, browser extension |

### Phase 1 — TPS multi-word conversion

`tl_numeric_to_tps` split on `-` only, while a continuous pick's romanization joins words with spaces (`tâi hó-lâng`), and `tps::to_zhuyin` takes one syllable — so `ㄉㄞˊ ho2 ㄌㄤˊ`. Fixed in the converter with the existing tokenizer (`tps::tl_syllable_tokens`, public as `phonetics::api::tl_syllables`), not a caller-side re-split: one tokenizer for every caller.

### Phase 2 — Bracket units always spaced (D2)

The engine answers `earns_auto_space`; the phones AND it with a platform predicate `spacesCandidateCommits(isAutoSpaceEnabled, isOutputBothScripts)` = either setting (the effective brackets value they already send), at every candidate / prediction commit site and in the swap. No engine or proto change: a `CommitResolution` field was rejected because the swap needs the platform-side value anyway. Scope (maintainer 2026-11-21): every candidate or prediction commit that earns a space, a Hanji-less one included (`gua 講 (kóng)`); the raw-preedit and English-word commits keep plain Auto-Space. Pinned: `behavioral-invariants.md` §23 `INVARIANT_BRACKETS_SPACE_CANDIDATE_COMMITS`.

### Phase 3 — Desktop key, settings rows and the D2 spacing gate

- desktop-core key `outputBothScripts` (default `false`, the iOS spelling) in `key_path_settings()` and `GENERAL_KEYS`; projection `SettingsDocument::effective_output_both_scripts()` = stored && `allows_output_both_scripts()` (shows Hanji, not TPS — D4), the name the mobile twins use. macOS, Windows and Linux show a switch under Output Script, greyed by `allows_output_both_scripts`.
- The gate ships with the key: desktop Auto-Space defaults to off, so the key alone would write `台語 (tâi-gí)我 (guá)`. Only a candidate commit takes `auto || effective brackets`; `CommitRaw` / `CommitAsShown` keep plain Auto-Space. The engine writes the space inside the transition, so the value travels as `ConfirmCompositionThenInsert.candidate_spacing_enabled` (`composing.proto`), read only when the pick finalizes the composition; macOS `ComposingBackend.swift` dropped its own Auto-Space check — the core decides.
- Upgrade: Windows and Linux have no retired-key sweep, so a hand-edited `outputBothScripts: true` left in `settings.json` takes effect on upgrade (accepted).
- Visible change: under TL / POJ with brackets on, same-Hanji dedupe is off (`continuous/dedupe.rs`), so two readings of one Hanji word show as two cells, as on mobile. Under TPS nothing changes (D4).

### Phase 4 — Web

A row beside the swap control (`web/src/settings.js`, disabled under Romanization Only and TPS). The ChromeOS and browser-extension options pages are cut from the same form (`web/chromeos/options-form.mjs`), which hides only rows those packages cannot honour, so the row shows there too (maintainer 2026-11-21).

## Best practices alignment

| Choice | Reference | Note |
|---|---|---|
| Bracketed second script | none | No IME in `../references/mainstream-ime-comparison.md` writes a bracketed second script |
| Fix in the converter, not a caller-side re-split | `AGENTS.md` § Design principles (no redundant fallback) | One tokenizer for every caller |
| Same key, same projection as mobile | `../contributing/cross-platform-alignment.md` | Intended behavior aligned, per-platform UI |

## Deliberately not adopted

- Per-word brackets inside one whole-sentence pick (D1): the commit would need word boundaries carried through, and the text gets busier.
- Two-line ruby output (#849, closed as not planned).
