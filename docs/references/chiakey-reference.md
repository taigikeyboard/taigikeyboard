# ChiaKey (千秋輸入法) Reference Research

> **Type**: Reference
> **Keywords**: `ChiaKey`, `KeyKey`, `OpenVanilla`, `Manjusri`, `LexiconContract`, `LearningStore`, `Updater`
> **Related**: mainstream-ime-comparison.md (card #26), ../architecture/macos-release.md, ../architecture/desktop-release.md, ../engine/sort.md, ../engine/custom-dictionary.md
> **Checkout read**: `references/ChiaKey/` @ `89aebc8c` (2026-09-18). Upstream `git@github.com:chiakich/ChiaKey.git`, BSD-style (Yahoo! 2012 + Chiaki.C 2026; no endorsement with the Yahoo! name). Read-only reference: cite, do not vendor.

---

## Summary

- macOS-only Bopomofo (注音) IME reviving the Yahoo! KeyKey / OpenVanilla lineage; Objective-C++ InputMethodKit host over a C++ engine (`OVIMMandarin` + `Manjusri` bigram language model over SQLite). ~60k LOC.
- The engine itself is not what we should learn from: Mandarin, Bopomofo, a Yahoo-era walker. Our Rust lattice, phonetics and ranking already cover that ground with better-documented references (McBopomofo, vChewing, khiin-rs).
- What **is** worth learning sits around the engine, and most of it targets exactly the desktop concerns we own on macOS / Windows:
  1. **Lexicon as a separately released, versioned, verified artifact** with atomic switch, runtime-driven prune / rollback and a bundled fallback (`Docs/LexiconContract.md`, `Scripts/install-lexicon-release.sh`).
  2. **Host-neutral core facade** shaped for multi-threaded desktop hosts: one `Runtime` per process, one `Engine` per text field, snapshot state, commit acknowledgement, C ABI (`Frameworks/ChiaKeyCore/`).
  3. **Bounded personal-learning store** with a stated eviction policy, corpus-measured capacities and an explicit, measured learned-bigram weight (`Frameworks/Manjusri/Headers/LanguageModel.h`).
  4. **Walker gold-set harness** that separates label noise from ranking error and measures "manual selections per pass" (`Frameworks/Manjusri/Tools/WalkerGoldSet.cpp`).
  5. **IME ↔ helper-app coordination without XPC**: lock file + dirty file + distributed notification + poll (`Loaders/OSX-IMK/ChiaKeyUserPhraseCoordination.h`, `ChiaKeyServiceCoordination.h`).
  6. **In-app updater trust chain**: CDN appcast first, GitHub API fallback, SHA-256, Developer ID team pinned to the installed build, `spctl` notarization assess, install lock (`Utilities/Updater/OSX/ChiaKeyUpdateService.m`).
  7. **Manual-dispatch release workflow** with `beta / patch / minor / major / stable` + `dry_run`, tag-derived version, secrets gate before the tag (`.github/workflows/release.yml`).
  8. **Large-list editor patterns** (windowed SQL paging, rowid deletes in one transaction, WAL) and **test hygiene** (`CFFIXED_USER_HOME` redirect, `-D` size caps) (`Docs/PhraseEditorRewrite.md`, `CONTRIBUTING.md`).
  9. **User-facing input features** on the product page (選字學習, 新增詞彙 `Shift+←`/`Ctrl+1…9`, 符號表, 簡體輸出 toggle, `Tab` 斷詞, 輸入模組) traced to source and compared with ours (§9). Two gestures we lack: in-buffer word capture and forced phrase break; both desktop-only, unscheduled.

---

## Architecture overview

```text
ChiaKey.app (macOS InputMethodKit host, Obj-C++)
  -> ChiaKeyCore facade            Runtime (per process) / Engine (per text field) / C ABI
  -> PlainVanilla loader           PVLoader / PVLoaderContext (candidate panel, filters)
  -> OpenVanilla modules           OVIMMandarin (SmartMandarin, TraditionalMandarin, AssociatedPhrase),
                                   OVIMGeneric (.cin tables: Cangjie, Simplex, user tables)
  -> Formosa                       Bopomofo syllable + keyboard layouts + reading buffer
  -> Manjusri                      SQLite bigram LM: Graph / Node walk, LearningStore, user DB
  -> ChiaKeySource.db              released by chiakich/ChiaKey-Lexicon, installed under
                                   ~/Library/Application Support/ChiaKey/Lexicons/active/
```

Side processes: Preferences app, Phrase Editor, Updater. None talk to the IME over XPC any more; they use files + `NSDistributedNotificationCenter` (§5).

| Dimension | ChiaKey | Taigi Keyboard |
|---|---|---|
| Engine language / host | C++ / Obj-C++ IMK, macOS only | Rust engine, Swift IMK / Rust TSF / Swift KeyboardKit / Kotlin |
| Segmentation | Manjusri `Graph::walk` bigram path search | `engine/composing/src/lattice` |
| Ranking | unigram + bigram log10 prob, learned overrides | `engine/ranking/src/score.rs` (user_freq, recency, closeness, base freq) |
| User adaptation | `user_bigram_cache`, `user_candidate_override_cache`, `user_context_override_cache`, `user_learning_stats` | `user_frequency.db` (count, last_used) + custom dictionary |
| Dictionary distribution | Separate repo + GitHub Release + CDN, verified at install, atomic symlink swap | `dictionary.bin` built in-repo, shipped inside every app build |
| App update | In-app updater, appcast on CDN, notarization check, `installer -pkg` | Manifest check only (`UpdateChecker.publishedURL`), no auto-update |
| License | BSD-style, no Yahoo! endorsement | — |

---

## 1. Lexicon release contract (highest-value takeaway)

**What ChiaKey does** (`Docs/LexiconContract.md`, `Scripts/install-lexicon-release.sh` 745 lines, `Docs/Architecture.md` § Runtime 資料權責):

- Dictionary lives in its own repo (`ChiaKey-Lexicon`); the generated `ChiaKeySource.db` is a **GitHub Release asset**, never committed to either repo. The app repo owns the **contract**: required tables, required metadata keys, minimum row counts, punctuation smoke checks (`_punctuation_<` must resolve to `，`), forbidden legacy keys.
- `lexicon-manifest.json` carries `version`, `database_schema_version`, `artifacts[{kind,url,filename,sha256}]`. The manifest is unsigned, so the installer also requires a **`SHA256SUMS` on a different origin** (GitHub release vs CDN) and refuses when the checksum artifact is missing. URL prefixes are allow-listed.
- Install layout is **versioned + symlink**: `Lexicons/versions/<ver>/` and `Lexicons/active -> versions/<ver>`; switch is `ln -sfn`, so any failure (download, checksum, SQLite integrity, missing table, row counts, plist parse) leaves the previous active untouched.
- Install writes a two-line `pending-verification` marker (new version, replaced version). **The runtime settles it**: opened fine → `--prune-superseded` (delete every non-active version, clear marker); open failed → `--rollback` (repoint `active` to line 2, delete the failed version, reload). `active` is the single source of truth for "which version is installed", so `--skip-current` never reads a version that cannot load. Local `dev` builds are exempt from prune.
- Runtime fallback order: external active → legacy external → bundled DB → legacy bundled. Bundled DB is required to be sufficient for offline rescue.
- Auto-update: at most one check per day; installs silently only when the latest release is **≥ 3 days old** (a bad release gets pulled before most users see it).
- User data separation is explicit: a lexicon release may refresh `canned_messages` (symbol table) but must never touch user phrases, learned ranking state or preferences; personal learning is an **overlay**, never an edit of the downloaded DB.

**Where we stand**: `dictionary.bin` / `association.bin` are built by `make dict` and shipped inside each platform build (`docs/architecture/build-artifacts.md`, `docs/architecture/data-artifacts-portability.md`). A dictionary fix therefore means an app release on every platform.

**Takeaway**: if a desktop dictionary channel independent of app releases is ever wanted, the checklist above (manifest + cross-origin checksum + versioned dir + atomic symlink + runtime-settled prune/rollback + bundled fallback + release-age delay + user-data overlay) is the complete design, already debugged in the field. The `pending-verification` two-line marker and "runtime settles, installer never deletes" split are the non-obvious parts. Relevant only if the USER opens that scope; not proposed here.

---

## 2. Host-neutral core facade (`Frameworks/ChiaKeyCore/`)

**What ChiaKey does** (`Headers/ChiaKeyCore/ChiaKeyCore.h`, `ChiaKeyCoreC.h`, `Source/ChiaKeyCore.cpp` 576 lines, `Tests/ChiaKeyCoreSmoke.cpp` 884 lines):

- `Runtime` = one per process: opens the DB once, builds the loader and static module packages, owns config. `Engine` = one per text field: wraps a `PVLoaderContext`, exposes only `handleKey`, `handleAsciiKey`, `selectCandidate(absoluteIndex)`, `reset`, `snapshot()`, `acknowledgeCommit()`.
- `Runtime` holds a **recursive mutex**; every `Engine` call goes through it, so an in-process multi-threaded host (TSF) needs no extra locking. Reshaped in 2026-09 after evaluating a Windows TSF and a Fcitx 5 front end.
- `EngineState` snapshot: `readingText`, `composingText`, `committedTextSegments`, `cursorPosition`, `highlight`, `wordSegments`, `tooltip`, `candidateState { candidates, contextPicks[], page, highlightedIndex }`, `beeped`, `notifications`.
- `acknowledgeCommit()` is a **host handshake**: committed text stays in the snapshot until the host confirms it inserted it, so a host that fails mid-commit does not lose text.
- `contextPicks` is a bool vector **aligned with `candidates`**, true when the preceding text (bigram) promoted the pick.
- Config is host-owned: every `Runtime::Create` reapplies the config the host passes; the core never touches platform preference stores. Changing the primary input method or associated-phrase flag **rebuilds every Engine context and drops in-progress composition** (documented on the API).
- A **CMake build** exists purely for non-Xcode hosts and runs the same smoke test; the build must define `WIN32` explicitly because MSVC only predefines `_WIN32` (`CONTRIBUTING.md`).

**Where we stand**: our Rust engine already is the platform-neutral core (`engine/composing` `EngineState` + `Effect`, `engine/protos`, `engine/swift-ffi`, `engine/android-jni`; contract in `docs/architecture/composing-state-boundary.md`). Windows TSF already runs it in-process.

**Takeaways**:
- The **per-process / per-field split with one runtime lock** is the same shape as our Windows `Runtime` + per-context state; worth citing when a TSF threading question comes up.
- **Commit acknowledgement** and **`contextPicks` alongside candidates** are two small API ideas: the first for any host where insertion can fail asynchronously (Electron / Chromium hosts, cf. #85), the second if the candidate UI ever wants to mark next-word-promoted candidates.
- "Config change drops composition" as a documented API guarantee is cleaner than reconciling mid-composition.

---

## 3. Bounded learning store with stated policy (`Frameworks/Manjusri/Headers/LanguageModel.h`)

**What ChiaKey does** (`LanguageModel.h` lines 95–345 `LearningStore`, 456–540 capacities and scores; `MJSRLearningCacheTables.h`; `Tests/TestLearningStore.cpp` 811 lines):

- Four user tables, all with a unique index on `qstring`: `user_bigram_cache(qstring, previous, current, probability)`, `user_candidate_override_cache(qstring, current)`, `user_context_override_cache(qstring, current)`, `user_learning_stats(store, qstring, selection_count, last_used)`. The table list is a single header-only array shared by both importers so backup / restore cannot disagree about what a backup replaces.
- `LearningStore<T>` is a capped in-memory map with **recency buckets per selection count**. Eviction = "fewest selections, then least recently used" = front of the lowest non-empty bucket, O(1), no scan. Memory is the source of truth, so an eviction also deletes the row on disk, otherwise the table grows without bound and reloads rows already dropped.
- Capacities are **measured**: against a 417k-token chat corpus a user needs ~3.7k distinct overrides and ~32k distinct bigrams; the old 200-entry stores forced 12k re-selections. Chosen: overrides 8000, bigrams 16000.
- **Context-keyed override before global override**: a correction is first stored keyed by `(previous reading, this reading)`; it becomes a context-free override only after it has proved itself in `c_overrideGeneralizationContexts = 3` different contexts. Measured: cuts wrong overrides of an already-correct lexicon answer by 22% while needing fewer manual selections than trusting unconditionally.
- **Learned-bigram weight is an explicit constant** (`LearnedBigramScore() = 0.0` = log10(1)) with the measurement that rejected weakening it recorded in the comment (weaker first pick: +11% manual selections, no held-out gain). "Users rely on one correction sticking."
- Reload races are handled explicitly: a row dropped in memory must not come back when `loadConfig()` re-reads tables mid-save; eviction on `setCapacity` sheds immediately rather than waiting for inserts.

**Where we stand**: `user_frequency(count, last_used)` keyed by `(word, tl)`, saturating boost + exponential decay (`docs/engine/sort.md`, `engine/ranking/src/score.rs`); next-word prediction scoring in `engine/nextword` (`scorer.rs`; the old candidate booster had no production caller and was removed 2026-09-25). macOS learning tables already self-trim (`LearningCapacity` 20000 / 50000 rows, least-used-first; `docs/architecture/macos-roadmap.md` § Settings pane roster). No context-keyed override tier.

**Takeaways**:
- **Eviction policy**: ours is count-based least-used-first; ChiaKey's adds a recency tiebreak inside each count bucket and shows how to keep it O(1). Same idea; the bucket structure is the only refinement, and only matters if trim cost ever shows up.
- **Context-keyed → generalized override** is the principled version of "one correction sticks, but does not poison other contexts". Relevant if a user-correction complaint ever comes in on the lattice path.
- Recording the *rejected* tuning with its numbers next to the constant is a documentation habit worth copying into `score.rs` constants.

---

## 4. Walker gold-set harness (`Frameworks/Manjusri/Tools/WalkerGoldSet.cpp`, `Scripts/eval-walker-goldset.sh`)

**What ChiaKey does** (`CONTRIBUTING.md` § 量測智慧注音 walker):

- `eval-` prefix, not `test-`: a comparison tool for ranking changes, not pass/fail.
- Gold set is derived from plain sentences, so the input reading sequence must be inferred, and every heterophone (多音字) is a chance to mislabel. `--dominance N`: `0` keeps only characters with a single lexicon reading (noise-free, short sentences, 1,182 rows); `1.0` also accepts characters whose top reading leads the runner-up by N log10 (~10,400 rows). Finding: the strict set could not detect harm from a length-prior change at all; the loose set reproduced the optimum. Rule: **report absolute accuracy only from the strict set; tune on the loose set**, printing the noise-free subset alongside.
- `replay` mode drives the real composer with learning enabled and counts **manual selections per pass** — the metric users feel — and writes learning, so it demands a scratch user DB.
- Gold set is never committed (local corpus may contain private chat).

**Where we stand**: `engine/composing/tests/candidate_dump.rs` produces before / after dumps; verdicts are qualitative (code-review rule §9: the USER is QA, no quantitative gate unless asked).

**Takeaway**: the same label-noise problem exists for Taigi (hanji → TL is ambiguous: `重/tîng` vs `重/tāng`, the (漢字, TL) identity in Core Principle #6). If a numeric ranking harness is ever requested, ChiaKey's strict / loose split and "selections per pass" replay are the design. Not a proposal; reference only.

---

## 5. IME ↔ helper coordination without XPC (`Loaders/OSX-IMK/ChiaKeyUserPhraseCoordination.h`, `ChiaKeyServiceCoordination.h`, `Docs/PhraseEditorRewrite.md`)

**What ChiaKey does**:

- Root cause of the rewrite: the NSXPCConnection migration never registered a launchd mach service, so the listener "started" but never received a connection. Rather than fix XPC, the maintainers removed the client entirely.
- Phrase Editor ↔ IME: three files next to the user DB. `SmartMandarinUserData.editing` (**lock**; editor creates on open, refreshes mtime, removes on exit; IME pauses auto-learn and queues `chiakey://` additions while it is fresh; **30-minute staleness** so a crashed editor never suspends writes forever), `SmartMandarinUserData.dirty` (mtime advances per commit; IME reloads caches when newer than last load), plus `NSDistributedNotificationCenter` as a best-effort hint. IME polls every 5 s as the lossless fallback.
- Lock contents = one PID per line so overlapping sessions (editor + Preferences import) do not tear down each other's lock; dead PIDs pruned via `kill(pid, 0)`; a sidecar `.editing.lock` is `flock`ed around every read-modify-write because an atomic file write does not make the pair atomic. IME reads existence + mtime only, so older single-PID builds interoperate.
- Preferences ↔ IME: the IME **publishes** `IMEStatus.plist` (version, DB version, modules, packages) after startup / reload / blacklist change; Preferences reads the file. Requests go the other way as notifications; a desired module blacklist is written to a pending file first so it survives the IME not running. Liveness is `NSRunningApplication`, not a ping. User data directory is created `0700`.
- WAL on open so the IME reads while the editor writes; no `VACUUM` during an editing session because rowid stability is assumed.

**Where we stand**: settings app ↔ IME on macOS goes through the shared engine `Runtime::update_settings` and the defaults / file layer described in `docs/architecture/macos-roadmap.md`; the Windows settings app (W17) is WinUI in a separate process.

**Takeaway**: for any future desktop helper that edits user data while the IME is live (custom-dictionary editor on macOS / Windows), the lock + dirty + notification + poll quartet with a staleness timeout is a proven minimal protocol; the PID-list + `flock` sidecar detail is what makes two helpers safe at once.

---

## 6. Updater trust chain (`Utilities/Updater/OSX/ChiaKeyUpdateService.m` 987 lines, `PreferenceApplications/OSX/TakaoUpdate.m`)

**What ChiaKey does**:

- Appcast JSON on a CDN is queried **first**; the GitHub Releases API is only a fallback, because the unauthenticated GitHub limit is 60/hour **per IP** and one corporate NAT exhausts it with daily checks.
- Before `installer -pkg`: SHA-256 from the manifest; `codesign` output must contain `Developer ID Installer`; the **team identifier is pinned to the installed app's team** so a different Apple developer account cannot ship an update; `spctl --assess --type install` must report `Notarized Developer ID`; an `flock` install lock prevents concurrent installs.
- Skip-this-version and opt-in beta channel live in a shared defaults suite; `CFBundleVersion` stays numeric and the full tag (`-beta.N`) rides in a separate `ChiaKeyReleaseTag` key so beta installs still see later betas.
- CI refuses to publish a release unless all signing / notarization secrets are present (checked **before** the tag is computed), because the updater would reject an ad-hoc build anyway.

**Where we stand**: `docs/architecture/macos-release.md` § Publishing: the shipped build reads a manifest (`version` / `downloadPageURL` / `packageURL`) and points the user at a download page; explicitly "No auto-update". Staging already checks stapling + Gatekeeper + `Distribution` bundle id.

**Takeaway**: if in-app install is ever added, the four-step verify (checksum → Developer ID → **team pin** → `spctl` notarized) plus install lock is the checklist; the CDN-first / API-fallback rationale applies to our manifest endpoint today.

---

## 7. Release workflow (`.github/workflows/release.yml`, `Docs/ReleasePackaging.md`, `Scripts/build-release-package.sh`)

- `workflow_dispatch` with `release_type ∈ {beta, patch, minor, major, stable}` + `dry_run` + `verbose`. Next tag is computed from existing tags (`git tag --sort=-v:refname` with `versionsort.suffix=-beta`); a beta after a stable bumps patch first so the prerelease sorts above it; existing tag aborts.
- Two `Info.plist`s bumped together (IME loader + Preferences) because Preferences displays its own bundle version.
- `pkgbuild` output is **rebuilt as a clean payload** because modern macOS turns the `com.apple.provenance` xattr into `._*` AppleDouble entries inside the package.
- Bundle ships a `Contents/Resources/Legal/` folder (LICENSE, COPYING, ACKNOWLEDGEMENTS, vendored-library notices) so binary redistribution keeps attributions.
- Release packaging **downloads the latest lexicon release** and verifies it; it never rebuilds the DB from source and never silently bundles a local DB (`--bundle-local-lexicon` is dev-only and labelled).
- Per-user install to `~/Library/Input Methods` by default; global via `sudo installer -target /`; `Scripts/uninstall.sh --purge`; localized installer resources (they note pkg localization did not take effect, open item).

**Where we stand**: `desktop-release.md` (staging + announcing gates), `macos-release.md` (two certificates, notarize, staple, `productbuild` from `Info.plist`, postinstall kills the live process), `release-desktop` skill. Same overall shape; no beta channel, no pre-tag secrets gate.

**Takeaways**: the provenance-xattr clean-payload step is worth checking against our `.pkg` (`pkgutil --expand` and look for `._*`); the `Legal/` folder is a tidy answer to the dictionary-licence position still open in `docs/go-public-checklist.md`.

---

## 8. Large-list editor + test hygiene (`Docs/PhraseEditorRewrite.md`, `CONTRIBUTING.md`)

- Phrase Editor targets tens of thousands of rows: fully **virtualized table** (windowed SQL paging + page cache), search / sort pushed to SQL (`LIKE`, reading → qstring prefix, `ORDER BY`), **rowid-addressed batch delete in one transaction** (50k rows in milliseconds; the old path copied the table per delete), default reading = most-likely reading per character instead of a cartesian product.
- Tests redirect `CFFIXED_USER_HOME` to a temp home and **refuse to run if the redirect fails**, so a test can never write into the real profile.
- Size caps are compile-time `-D` macros so the cap test builds a second binary with a tiny limit instead of writing 192 MB; file-size caps use a sparse file at the shipped value.
- Legacy import golden vectors contain only the SQLite header + nonce (no phrase data) yet still pin key derivation, mode, IV position and counter rules.

**Where we stand**: custom dictionary CRUD + CSV (`docs/engine/custom-dictionary.md`); mobile lists are small; no desktop editor.

**Takeaway**: paging + rowid-delete pattern if a desktop custom-dictionary editor ever grows past a few thousand rows; the `CFFIXED_USER_HOME` guard is directly reusable in any macOS test that touches `~/Library`.

---

## 9. User-facing input features (from https://chiaki.ch/works/chiakey, fetched 2026-09-19)

The product page lists six features. Each is traced to source below and compared with what Taigi Keyboard ships on desktop (macOS IMK / Windows TSF) and mobile.

| # | Feature as advertised | ChiaKey mechanism (`file`) | Taigi Keyboard today | Transfer? |
|---|---|---|---|---|
| 1 | **選字學習** — context + selection history rank candidates; frequently chosen rise | Two tiers on every candidate pick (`OVIMSmartMandarin.h::chooseCandidate`): (a) **context-keyed override** keyed by `(previous reading, this reading)` — BOS counts as a context so sentence-initial picks are learnable; promoted to a context-free override only after 3 distinct contexts (`LanguageModel.h` `c_overrideGeneralizationContexts`); a pick that equals the lexicon's first candidate *clears* a stale override instead of storing one; (b) **user bigram** `(previous text, this text)` at `LearnedBigramScore = log10(1)` so one correction sticks. Punctuation / passthrough / ctrl keys never learn. Learning is suspended while the Phrase Editor lock is fresh. | `user_frequency` count + recency (`engine/ranking`), next-word association write-only on macOS (`macos-roadmap.md` § learning), next-word predictions on mobile (`engine/nextword`). Single-tier, context-free. | The **"pick equals lexicon default → clear override"** rule and the **context-keyed → generalized** promotion are the two ideas; both are lattice-side and only relevant if a user-correction complaint arrives (Core Principle #4, reactive). |
| 2 | **新增詞彙** — `Shift+←/→/Home/End` marks characters in the composing buffer, `Enter` adds; or `Ctrl+1…9` adds the preceding N characters | Mark mode in `OVIMSmartMandarin.cpp` ~370–475: only when no reading is pending; span capped at **6 characters**; highlight drawn via `PVTextBuffer::setHighlightMark`; tooltip 「正在選取字詞組：X，請按 ENTER 鍵加入資料庫」; `Esc` / bare arrow cancels; any other key beeps and drops the mark. `Ctrl+N` path `handleQuickUserUnigramKey` ~213: beeps if fewer than N characters precede the cursor. Both call `addUserUnigram(qstring, current)` (`LanguageModel.h` ~1700), which rejects duplicates already in the system or user unigram table and refuses while the editor lock is fresh. Added word = **(reading sequence, text) pair**, i.e. the same `(reading, 漢字)` identity as our Core Principle #6. | Custom words are added in the settings app (`自訂詞庫` pane on macOS; iOS / Android custom-dictionary screens), not from the composing buffer. On mobile, S50 covers 披頭巾 via custom word. | **In-buffer word capture** is the one desktop input feature we lack. The `Ctrl+N` form is the cheaper of the two (no highlight rendering, no mark state machine). Our `⌃1…9` chord is already taken as one of the candidate slot-key picker choices (`macos-roadmap.md` D4), so a desktop version would need its own chord. Not proposed; recorded as the reference. |
| 3 | **符號表** — `Ctrl+Cmd+.` opens punctuation / special-character panel | IMK menu item key equivalent (`OpenVanillaController.mm` ~1448) → `CVSymbolController.mm`: categories from `canned_messages` plist in the lexicon DB (merged into user persistence DB on lexicon update); ~1000 `NSButton`s built lazily per category because keeping all resident was measured as too heavy. | macOS symbol picker exists (#85 placeholder fix, #88 recent-picks lead + ⋯); categories are ours, not lexicon-supplied. | Already have it. Two details worth noting: **recent picks first** we already did in #88; **lazy per-category build** matches the leak-free / 64 MB posture. Nothing to add. |
| 4 | **簡體輸出** — `Ctrl+Cmd+G` toggles an output filter: type Traditional, commit Simplified | `OVOFHanConvert` output-filter module (`ModulePackages/OVOFHanConvert/`, table-driven TC2SC); toggled per identifier via `toggleOutputFilterAction:` (`OpenVanillaController.mm` ~1225); output filters apply at commit only, so composing text stays Traditional. Companion filters: `OVOFFullWidthCharacter` (`Cmd+Shift+Space`), `OVOFAntiZhuyinwen`. **Shift tap** toggles a *temporary English* mode (`_setTemporaryEnglishMode`, ~650–690) with a minimum-hold guard so a modifier resync is not mistaken for a tap. | Output script picker in 一般 (#86: 漢字 / TL / POJ / TPS, default 漢字) — same "input stays, output converts" shape, chosen in settings rather than a chord. | **Commit-time output filter as a toggleable chain** is the generalization of our output picker; the Shift-tap English toggle with a hold-duration guard is a pattern to remember if a "temporary Latin" mode is ever asked for. |
| 5 | **Tab 斷詞** — `Tab` at the cursor forces a phrase break; whole sentence re-walks | `OVIMSmartMandarin.cpp` ~543 → `Graph::toggleForcedBreakAt` (`Graph.h` ~1178): a `set<size_t>` of break indices; `crossesForcedBreak` vetoes any node spanning a break during `rebuild`; pressing `Tab` again at the same boundary **removes** the break (added because a silent no-op left no way out short of clearing the buffer). Beeps while a reading is pending or the buffer is empty. | No forced-break gesture. Our continuous input re-segments on each keystroke (`engine/composing/src/lattice`, `continuous.rs`); the user corrects by picking a candidate at a span, and mobile has no Tab. | **Toggle semantics** (second press undoes) is the non-obvious part. Desktop-only idea; relevant only if a "walker picked the wrong boundary" report arrives on macOS / Windows. |
| 6 | **輸入模組** — smart Zhuyin default; traditional Zhuyin, Cangjie, Simplex (簡易) kept; user `.cin` tables (Cantonese, Hakka, …) importable | `OVIMTraditionalMandarin` (per-syllable, no LM), `OVIMGeneric` over `.cin` (`OVCINDataTable.h`), user tables at `Application Support/ChiaKey/DataTables/Generic/`, imported through `TakaoCINTable.m` with a preview (name, count, selection keys) and Big5 → UTF-8. Six Bopomofo keyboard layouts (`Formosa/Mandarin.h`: Standard, ETen, Hsu, ETen26, IBM, HanyuPinyin). | Single engine; layouts are TL / POJ / TPS input modes, not pluggable tables. | No transfer. Multi-module loader is exactly what "Deliberately not adopted" says. |

Other page claims, for the record: lexicon layers (base characters, contextual correction, trending terms, Taiwan-usage adjustment, personal dictionary) built from ~470 M characters of Taiwanese text — that is `ChiaKey-Lexicon`, outside this repo; macOS 10.13+ and Apple Silicon native. The `OVAFHomophoneLookup` (backtick), `OVAFReverseLookup`, `OVAFBopomofoCorrection`, `YKAFPhraseAware` and `YKAFWordCount` around-filters exist in the tree but are not advertised on the page.

**Net for Taigi Keyboard**: of the six, four we already have or deliberately do not want (3, 4, 6, and the learning in 1 in a simpler form). The two genuinely missing input gestures are **in-buffer word capture** (2) and **forced phrase break** (5); both are desktop-only and both stay unscheduled until a USER-reported need (Core Principle #4).

---

## Smaller notes

- **Generic `.cin` table import** (`ModulePackages/OVIMGeneric`, `PreferenceApplications/OSX/TakaoCINTable.m`): user tables live under `Application Support/ChiaKey/DataTables/Generic/`, Big5 auto-converted, preview (name, entry count, selection keys) before import, explicit warning never to drop files into the app bundle (breaks the signature, overwritten on update). Same "external tables outside the bundle" rule our user data already follows.
- **Associated phrases** (`ModulePackages/OVIMMandarin/OVAFAssociatedPhrase.cpp`): head-character keyed `associated_phrases` SQLite table, offered after commit. Simpler than our next-word prediction; only the "toggle rebuilds contexts" contract is notable (§2).
- **Docs discipline**: `Architecture.md` carries `狀態：已採納` + dated decisions, `ModernizationPlan.md` has an explicit `延後事項` list, `PhraseEditorRewrite.md` is a postmortem with the landmines listed, and the AI-agent contribution is disclosed with the review scope. Same spirit as our `docs/architecture/incident-log.md` + `behavioral-invariants.md`.
- **Yahoo legacy import**: the old encrypted DB can only be read by the old IME, so ChiaKey asks the *running* legacy IME to export (needs Rosetta 2). Historical curiosity; no transfer.

---

## Deliberately not adopted

| ChiaKey pattern | Why not for Taigi Keyboard |
|---|---|
| Obj-C++ InputMethodKit host + OpenVanilla module loader | Our macOS host is Swift over the Rust engine (`docs/architecture/macos-roadmap.md`); module plug-in system is YAGNI for one engine. |
| SQLite-resident bigram LM queried at keystroke time | Our lexicon is an FST + mmap binary (`docs/engine/binary-format.md`); ranking is in `engine/ranking`. |
| Bopomofo reading buffer (`Formosa`) | Phonetics are TL / POJ / TPS from `knowledge/taigi-phonetics-reference.md`; Tekkon is the closer syllable-composer analogue (card #25). |
| Separate dictionary repo + OTA lexicon | Dictionary is built in-repo and shipped with the app; changing that is a USER-gated scope decision (§1 lists the design if it is ever opened). |
| In-app auto-update | `macos-release.md` records "No auto-update" as the current decision. |

---

## Where to look

| Topic | Path under `references/ChiaKey/` |
|---|---|
| Architecture + decisions | `Docs/Architecture.md`, `Docs/ProjectStructure.md`, `Docs/ModernizationPlan.md` |
| Lexicon contract + installer | `Docs/LexiconContract.md`, `Scripts/install-lexicon-release.sh` (rollback at ~148, prune ~202, SHA256SUMS cross-check ~612, symlink swap ~716) |
| Core facade | `ChiaKey-Source/Frameworks/ChiaKeyCore/Headers/ChiaKeyCore/ChiaKeyCore.h`, `ChiaKeyCoreC.h`, `Source/ChiaKeyCore.cpp`, `Tests/ChiaKeyCoreSmoke.cpp` |
| Learning store + LM | `ChiaKey-Source/Frameworks/Manjusri/Headers/LanguageModel.h`, `MJSRLearningCacheTables.h`, `Tests/TestLearningStore.cpp` |
| Walker + eval | `ChiaKey-Source/Frameworks/Manjusri/Headers/Graph.h`, `Node.h`, `Tools/WalkerGoldSet.cpp`, `Scripts/eval-walker-goldset.sh` |
| Process coordination | `ChiaKey-Source/Loaders/OSX-IMK/ChiaKeyUserPhraseCoordination.h`, `ChiaKeyServiceCoordination.h`, `Docs/PhraseEditorRewrite.md` |
| Updater | `ChiaKey-Source/Utilities/Updater/OSX/ChiaKeyUpdateService.m` (CDN-first ~439, team pin ~683, `spctl` ~707) |
| Release / packaging | `.github/workflows/release.yml`, `Docs/ReleasePackaging.md`, `Scripts/build-release-package.sh`, `Packaging/Installer/` |
| Candidate window (IMK, Obj-C) | `ChiaKey-Source/Loaders/OSX-IMK/CVHorizontalCandidateController.mm`, `CVVerticalCandidateController.mm`, `CVSymbolController.mm` |
| Tests + scripts | `Scripts/test-*.sh`, `CONTRIBUTING.md` § 本機測試 |
