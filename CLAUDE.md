# CLAUDE.md

Guidelines for **Claude Code** in this repo. Two-repo AI environment:

- **Cross-project process rules** (workflow, planning, diagnosis, review, naming, docs authoring) live in `~/.claude/rules/` — managed by the [`configurations`](https://github.com/siansiansu/configurations) dotfiles repo and symlinked in by its `setup.sh`. **Fresh machine**: clone `configurations` + run its `setup.sh` before working here.
- **Project-specific rules** live in `.claude/rules/` — auto-load via `paths:` glob when Claude reads matching files; three are always-on (`security-rules.md` / `doc-lookup.md` / `taigi-incidents.md`).
- **Personal defaults** (theme, response style, Codex usage) live in `~/.claude/CLAUDE.md` (also from `configurations`).

## Project Overview

**Taigi Keyboard** — cross-platform Taiwanese input method: iOS (Swift + KeyboardKit), Android (Kotlin + FlorisBoard), macOS (IMKit) and Windows (TSF) over a shared Rust engine. POJ/TL romanization, Hanji (漢字), tone variation, autocomplete, continuous input.

## Project Structure

```
taigikeyboard/
├── android/          # Android app (Kotlin + FlorisBoard)
├── ios/              # iOS app (Swift + KeyboardKit)
├── macos/            # macOS input method (Swift + IMKit, SwiftPM)
├── windows/          # Windows input method (Rust TSF, Cargo workspace)
├── engine/           # Shared Rust engine — Cargo workspace, FFI to every platform
├── docs/             # Specs: engine/, architecture/, ui/, references/, reports/, roadmap.md
├── knowledge/        # Taiwanese phonetics reference (TL/POJ/TPS)
├── taigi-converter/  # Canonical TL↔POJ↔TPS converter (git submodule)
├── .claude/rules/    # Project rules (auto-load via paths: glob) — see "Mandatory Rules"
├── dictionary/       # Dictionary sources + build pipeline + output artifacts
├── changelog/        # Per-release changelogs — edit only at release time
├── content/          # In-app content (FAQ / feature JSON)
└── references/       # Cloned external IME repos (gitignored)
```

## Core Principles (project-specific)

1. **No project-config modification by AI** — `.xcodeproj` / `.pbxproj` / `.xcworkspace` are **user-only** (enforced by `.claude/hooks/block-project-config.sh`). Xcode 16 synchronized groups auto-include new files under most `Sources/TaigiKeyboard/*` subdirs — rules + exceptions in `.claude/rules/ios-guidelines.md`. Android Gradle (`build.gradle`, `*.gradle.kts`) **is** editable (lifted 2026-05-09).
2. **Cross-platform alignment** — align on **intended behavior**, not API calls: define expected behavior, verify each platform independently, document when the same behavior needs different implementations.
3. **Phonetics = authoritative-source-only** — never infer TL/POJ/TPS rules (or "dead" phonetic tables from test/dictionary absence); read `knowledge/taigi-phonetics-reference.md` and consult `taigi-converter/` first. Read-order in `.claude/rules/phonetics.md`.
4. **Bugfix = confirm root cause before fixing** — trace and verify the root cause (cite `file:line`, evidence), present it, and **wait for explicit approval**. No branch, no edit, no implementation until the user agrees. Diagnosis and fixing are separate, sequential, user-gated steps.
5. **Release scope / timing / tag = user-gated** — never decide what is in/out of vX, never write "deferred / post-vX / known limitation / ready to tag" without the user's explicit dated words. Present work factually (cost, options, trade-offs). Full rule: `~/.claude/rules/diagnosis-discipline.md` § No unilateral release scope; incident: `.claude/rules/taigi-incidents.md`.
6. **Direction-first over fix-scope** — between two correct fixes, prefer **consistency, best-practice alignment, and correct architectural direction** over the smaller change-set. If the smaller fix (per-call-site qualification, one-line workaround) preserves a naming inconsistency / anti-pattern / recurring trap, take the larger fix that resolves the root cause. State the trade-off; do NOT default to minimum-change. **Why**: USER 2026-05-25 — "比起修復範圍,我認為考慮一致性、最佳實踐,方向正確會比較重要" (iOS `AutocompleteService` ambiguity round: per-call-site qualification was smallest, full class rename was the root-cause fix).
7. **Taiwanese word identity = (漢字, 羅馬字) pair** — a word is identified by the **combination** of its Hanji AND its canonical-TL reading; **neither alone is a key**. 一字多音: same Hanji + different reading = different morpheme (`重/tîng` 重複 vs `重/tāng` 重量; `八/pat` 文讀 vs `八/pueh` 白讀); homophones: same reading + different Hanji = different word. Governs **every** dedup / lookup / ranking-merge / accent / variant / substitution decision **project-wide** (engine `lexicon`/`ranking`, dictionary `merge`/`merge_csv`/`cleanup`, accent-substitution generation). Key on the `(hanzi, tl)` pair — never `hanzi` alone, never `tl` alone. POJ/TPS are alternate renderings of the same TL reading, not separate identities. **Why**: USER 2026-05-29 — "針對台語羅馬字,(羅馬字 + 漢字)的組合視為同一個字".

## Mandatory Rules

Cross-project process rules auto-load from `~/.claude/rules/` (don't duplicate them here). Project rules in `.claude/rules/` auto-load on matching file reads; this table is the **topic index** for tasks framed by topic rather than by file path.

| Before… | Read |
|---|---|
| a change affecting iOS/Android parity | `.claude/rules/cross-platform-alignment.md` |
| modifying iOS code (structural → also architecture) | `.claude/rules/ios-guidelines.md` (+ `ios-architecture.md`) |
| marking a file as iOS Shared-Core Candidate or changing the candidate roster | `.claude/rules/ios-shared-core-candidates.md` |
| modifying iOS Settings wiring (`SharedSettings`, `EngineSettingsProvider`, live-read regressions) | `.claude/rules/ios-settings-injection.md` |
| modifying Android code (core architecture, Kotlin idioms, DI, DataStore) | `.claude/rules/android-guidelines.md` |
| modifying Android Compose / IME-specific code, testing, or a refactor-round PR | `.claude/rules/android-ime-patterns.md` |
| modifying app UI | `.claude/rules/ui-style-guide.md` |
| adding logging / SQL / network / storage | `.claude/rules/security-rules.md` |
| adding, renaming, or deleting an `i18n/*.json` key | `.claude/rules/i18n.md` — keys are cross-platform; deleting one can break another platform's build on `main` |
| Rust engine code (general hygiene, workspace, errors, crates, tests) | `.claude/rules/rust-best-practices.md` |
| Rust FFI / proto boundary code, `unsafe` blocks, opaque handles, enforcement | `.claude/rules/rust-ffi-safety.md` |
| starting a Rust slice migration / platform→engine swap / `.proto` addition / mirror-source delete | `.claude/rules/rust-migration-policy.md` |
| any TL/POJ/TPS schema, FST key-family, encoding, or canonical-form work | `.claude/rules/phonetics.md` |
| revisiting a global rule and wanting the concrete Taigi "why" | `.claude/rules/taigi-incidents.md` |
| preparing a real-device dogfood pass, or a PR / memory cites an `Sn` item | `docs/architecture/dogfood-checklist.md` |
| adding or changing a call to / contract with a framework/OS API (KeyboardKit, `UIInputViewController`/`UITextDocumentProxy`, `InputMethodService`/`InputConnection`/`EditorInfo`, Jetpack Compose, DataStore) — not trivial edits to framework-adjacent code | `.claude/rules/doc-lookup.md` — verify the current API via `find-docs`/`ctx7` (or local `references/KeyboardKit-Documentation/`) **before** coding; never from model memory |
| writing a 最佳實踐對齊 section, claiming "Project X does Y", or **designing OR fixing** a segmentation / lattice / ranking / user-freq / syllabifier / predictive / next-word / continuous-input slice | `docs/references/mainstream-ime-comparison.md` first (TL;DR matrix + topic index → per-repo cards; do **not** re-explore `references/` from scratch) |

## Build & Test

The **user runs all builds/tests manually mid-round** — never invoke these or add build hooks/reminders mid-round. **Commit-first ordering**: commit → push → `gh pr create` with NO pre-commit test gate; the post-PR parallel verification (below) fires AFTER `gh pr create` returns, in parallel with the PR-bot review. Do NOT block commit / push / PR-open on test results.

| Platform | Build | Test |
|---|---|---|
| iOS | Xcode → keyboard extension | `xcodebuild -project ios/TaigiKeyboard.xcodeproj -scheme TaigiKeyboardTests -destination 'platform=iOS Simulator,id=81ADB050-5242-460C-90DA-F3FAF3F6AAA5' test` (iPhone 17 / iOS 26.1, UDID-pinned for cache reuse; project `IPHONEOS_DEPLOYMENT_TARGET = 26.1`) |
| Android | `cd android && ./gradlew :app:assembleDebug` | `cd android && ./gradlew :app:testDebugUnitTest` |
| engine | `cargo build --workspace` | `cargo test --workspace` |
| macOS | `make -C macos build` (`make -C macos install` before a dogfood pass) | `make -C macos test` |
| Windows | `make windows-check` (host-side gate: pure-crate tests + clippy against the Windows targets; the TSF DLL itself only builds on the Windows box, see `docs/architecture/windows-release.md`) | included in `windows-check` |
| taigi-converter | — | `npm test` from `taigi-converter/` (bare `node --test tests/` fails on Node 26 with `MODULE_NOT_FOUND`) |

**Bootstrap after cloning** — the engine binaries a platform build links are generated, not committed. A fresh clone does not build until this has run:

| Command | Produces | Needed by |
|---|---|---|
| `make build` (5 s warm; minutes on a cold target dir) | iOS + macOS xcframeworks and their swift-bridge wrappers, Android `jniLibs/*.so`, platform protos | Xcode, SwiftPM, Gradle all link these |

The typefaces are committed, once, at `fonts/font/` — all four platforms package that directory (Android through a `res` source dir in `android/app/build.gradle.kts`, the other three by copying it), so nothing has to be staged before a build.

**Clone with `--recurse-submodules`**, or run `git submodule update --init --recursive` before `make dict`. `taigi-converter` is a submodule and the dictionary pipeline converts every reading through it; `make dict` and `dictionary/common/taigi_bridge.py` both refuse to start without it.

The dictionary artifacts are committed, once, at `dictionaries/` — all four platforms package that directory the same way the typefaces are (Android through an `assets` source dir, iOS through an Xcode synchronized folder, macOS and Windows by copying it), so nothing has to be staged before a build. They stay committed at all because that is the USER's standing instruction (2026-09-07: 「dictionary/ folder 都不要碰」), not a technical limit; `make dict` does reproduce them from a clean checkout: `dictionary/build.sh` writes them into `dictionary/output/`, where the four shipped files are untracked scratch (`dictionary.csv`, `corpus_total_freq.txt` and `.build_ts` there stay tracked), and `dictionary/build/deploy.sh` then copies them to `dictionaries/`.

**Ignored files survive `git checkout`, so this is one pass per machine, not per build.** After that, re-run only what a change invalidates — the table below.

**Stale-artifact gate (mandatory before every iOS/Android build+test)** — the artifacts above are local, untracked build output. They do not update themselves when a change lands, so building against a stale one gives **false-green tests**. Check `git diff --stat` against this table first:

| If the diff touches… | Run first | Regenerates |
|---|---|---|
| `engine/` (any Rust source, `.proto`, `Cargo.toml`) | `make build` | Platform protos + iOS/macOS xcframeworks + Android jniLibs |
| `dictionary/` (CSV sources, build scripts, syllabifier rules) | `make dict` then `make build` | `dictionary.bin` + `syllables.fst`, then the bundles above |
| platform-only Swift / Kotlin / docs | — | nothing |

`make build` is sequential — about 5 s against a warm target directory, minutes when it has to compile the engine for all five targets. `make dict` is a separate ~2 min pass that must finish first (measured 2026-09-07: 67 s of per-source pipelines, 53 s of aggregate build).

**EXCEPTION — a release.** `/release-mobile` and `/release-desktop` run `make i18n` + `make build` themselves (and `make dict` when dictionary sources moved): the engine binaries a platform links are generated and gitignored, so nothing else can prove the shipped artifact was built from the commit being released.

**EXCEPTION — post-PR parallel verification** (`~/.claude/rules/round-workflow.md` § Codex review sandwich step 6): immediately after `gh pr create` returns the URL, run build+test for **every platform the diff touches** in the background (one message, parallel `Bash` calls with `run_in_background: true`). If the diff touches `engine/` or `dictionary/`, the stale-binary gate runs FIRST (sequentially), then the platform gates fire in parallel. On failure: report the failing target + first error line, push the fix as a new commit on the same branch (no `--amend`), re-run only the failing gate. Do NOT close the PR.

## Communication

- Reply in **Taiwanese Mandarin (台灣華語)**; documentation and code comments stay in **English**.
- **Doc-authoring language**: living reference (`.claude/rules/**`, `docs/architecture/**`, `docs/engine/**`, `docs/roadmap.md`) defaults to **English prose**; CJK only for (a) verbatim USER quotes kept as evidence and (b) domain phonetic terms / examples. Dated snapshots (`docs/reports/**`, `docs/releases/**`) are frozen — do **not** retro-translate.
- Concise, bullet-point, key points only — no filler.
- Analyze first and present options; explain scope of impact before changing code.

## PR numbers before the 2026-09-07 migration

This repository was recreated on 2026-09-07 to shed pull-request refs that still
carried bug reporters' personal data. The full commit history came across intact;
the pull requests did not, and **numbering restarts at #1**. Every `#NNN` in
`docs/`, `changelog/`, `.claude/`, and project memory written before that date
refers to the old repository, `taigikeyboard/taigikeyboard-archive` (private).

Resolve an old number from git first — 528 squash-merge commits carry it in the
subject, and the commit is the diff:

    git log --all --oneline --grep="(#NNN)"

Only open the archive repository when you need the review discussion itself:

    gh pr view NNN -R taigikeyboard/taigikeyboard-archive

Old and new numbering will eventually collide. When a number resolves to two
different things, the archive is the older one.

## Key References

- `docs/README.md` — full documentation index
- `docs/roadmap.md` + `memory/project_*.md` — live multi-PR plan & round hand-off (this project uses these, **not** `IMPLEMENTATION_PLAN.md`)
- `references/` — cloned IMEs (azooKey, librime, khiin-rs, McBopomofo, florisboard, …); enter via the comparison doc above, not directly
- Android IME: follow [Creating an Input Method](https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method)
