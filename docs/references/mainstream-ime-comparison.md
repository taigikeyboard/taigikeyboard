# Mainstream IME Comparison

> **Type**: Reference index
> **Purpose**: Centralised cross-reference of every mainstream IME / keyboard repo cloned under `references/`, plus a few external projects worth knowing. Read this **before** writing a `最佳實踐對齊` section in a plan, before designing a new engine slice, or before asserting "Project X already does Y".
> **Status**: Authoritative as of 2026-05-17. Update when adding a new repo under `references/` or when an existing deep-dive doc lands in `docs/references/`.
> **Related deep-dives**:
> - [`azookey-reference.md`](./azookey-reference.md) — azooKey iOS UI / CustardKit / action model
> - [`khiin-reference.md`](./khiin-reference.md) — khiin-rs DP segmentation + bigram + dual-trie
> - [`rime-reference.md`](./rime-reference.md) — librime Pipeline / Spelling Algebra / user-dict decay
> - [`moe-taigi-reference.md`](./moe-taigi-reference.md) — MOE Taigi InputLine / Nail / segmentation
> - [`moe-taigi-asr-reference.md`](./moe-taigi-asr-reference.md) — MOE Taigi ASR sidecar

---

## How to use this file

1. **TL;DR matrix** below gives one row per repo + the dimensions Taigi Keyboard cares about (segmentation, ranking, user adaptation, tone handling, license, platform).
2. **Topic index** maps "I am working on X" → "read these repos in order".
3. **Per-repo cards** give a 1-screen summary of *what / why we care / where to look*. They are intentionally short. For depth, follow the deep-dive link if one exists, otherwise grep the path listed under "Where to look".
4. **External pointers** at the bottom list important IMEs not under `references/` — clone them on demand.

> ⚠️ This file is a **snapshot**. Each repo's upstream evolves. Before quoting line numbers in a PR, `git -C references/<repo> log -1 --format='%H %ad'` to record the SHA you read, or re-grep on the current checkout.

---

## TL;DR matrix

| # | Repo (under `references/`) | Lang | Platform | Type | Segmentation / Lattice | Candidate ranking | User adaptation | Tone / Syllabifier | License | Why we keep it |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `azooKey/` | Swift / SwiftUI | iOS (KeyboardExt) | Full IME (Japanese) | Kana → kanji DP with Mozc-style cost | Frequency + custom dict + neural LM (`zenz-v3`) | Commit history → personal dict; on-device | Kana syllable rules | MIT (azooKey core) | UI vocabulary, action model, Flick/QWERTY dual-track. **Deep-dive**: `azookey-reference.md` |
| 2 | `azooKey-Desktop/` | Swift | macOS (IMK) | Full IME | Same engine as azooKey iOS, exposed via Input Method Kit | Same | Shared user dict format | Shared | MIT | macOS IMK integration patterns; useful if Taigi ever ships Mac |
| 3 | `CustardKit/` | Swift + JSON | Cross (data format) | Layout DSL | n/a | n/a | n/a | n/a | MIT | Declarative custom-tab JSON schema. Reference for any "export keyboard layout to file" feature |
| 4 | `keyboardkit9.9.0/` | Swift | iOS (framework) | Framework | n/a (host-defined) | n/a | n/a | n/a | MIT (open) + commercial Pro | What our iOS keyboard *runs on top of*. Vendor SDK; consult `KeyboardKit-Documentation/` first |
| 5 | `KeyboardKit-Documentation/` | (markdown) | iOS | Docs | — | — | — | — | MIT | Authoritative KK API docs; treat as gospel before WebSearching |
| 6 | `florisboard/` | Kotlin | Android (IME) | Full keyboard | NLP module gutted in OSS (`libnative/dummy`) — v0.6 milestone | Suggestion engine **not shipped** in OSS | Clipboard history, theme | n/a (text layout only) | Apache 2.0 | **Architecture mirror** for our Android side. Subclassing patterns, IME service skeleton, snygg theming DSL, Compose IME UI. Engine logic *not* here |
| 7 | `Hamster/` | Swift + librime (C++) | iOS | RIME wrapper | Delegates to librime | librime's Spelling Algebra | librime user dict (LevelDB) | librime syllabifier | MIT (v2.1.0+) | How to **embed librime as an iOS framework**; LibrimeKit GitHub Actions build pattern. Closed-source from "Cang" v3+ |
| 8 | `librime/` | C++ | Cross (desktop) | Engine core | DAG segmentation (Aho-Corasick over prism) + Viterbi | Spelling Algebra penalties + LevelDB user dict with dynamic weight + time decay (`exp((ta-t)/200)`) | LevelDB key-value: `commits / dee / tick` | Per-schema YAML (e.g. luna_pinyin) | BSD | Industry-canonical reference for **modular IME pipeline** and **user dict math**. **Deep-dive**: `rime-reference.md` |
| 9 | `librime-predict/` | C++ | Cross | RIME plugin | n/a | Standalone next-word predictor table (`predict.db`) | Reads but does not write user state | n/a | BSD | Minimal predictor reference — fewer than 10 source files. Useful if we ever ship a configurable next-word switch instead of always-on |
| 10 | `rime-moetaigi/` | YAML + dict | Cross (RIME schema) | Schema data | DAG inherits from librime | Frequency from MOE corpus + 楊允言 word stats | Auto user-phrase via librime | TPS (Bopomofo-style) + 9 tone marks | CC0 | **Only mainstream RIME-based Taigi schema**. Reference for keyboard layout, tone-mark placement, 中文反查 mode, 注音顯示 mode |
| 11 | `McBopomofo/` | Swift + Obj-C++ + C++ | macOS (IMK) | Full IME | **Gramambular**: ReadingGrid + Span + Node, DAG with topological sort + relaxation | Unigram log-prob + Max-Match weighting (`fscale=2.7`) + heterophone penalties (×0.5 / ×0.25 / floor) | UserPhrasesLM + ExcludedPhrases; **per-syllable epsilon boost** prevents single-char overpowering multi-syllable | Bopomofo syllable validator | MIT | Cleanest open-source **Mandarin IME with deep algorithm doc**. `algorithm.md` is mandatory reading before touching our Phase 9 lattice. The epsilon-boost trick (`McBopomofoLM.cpp:120`) is directly relevant to Taigi user_freq_boost |
| 12 | `khiin-rs/` | Rust | Desktop (Win/Linux) | Full IME (Taigi) | Word-level DP over `cost_map` with `ln(1/p^FREQ)/word_len^LET * n_syls^SYL` | Unigram + Bigram (`lgram, rgram, n`) | SQLite `unigrams` + `bigrams`; learning on commit | POJ/TL syllable regex + Telex tones (`s/f/l/j`) | MIT | **Most architecturally aligned with us**. Three input modes (Continuous / Classic / Manual) are the model we mirrored in Phase 8. **Deep-dive**: `khiin-reference.md` |
| 13 | `moe_taigi_apk/` (decompiled) | Java (decompiled from C++) | Android (IME) | Full IME (Taigi, MOE official) | "Nail"-based segment-by-segment commit; `InputLine` + `Segmentation` | Closed-source C++ via SWIG JNI | `UserVoc` + `LearnedVoc` separated; `RIPE_*_APPROVALS` maturity thresholds | TL only; binary `tailo.tab` syllable trie | Closed-source binaries; AGPL-equivalent terms unclear | **Reference Taigi UX baseline**. Nail commit flow, span-units, Han-lo mixed candidates (`VT_MIXED`). **Deep-dive**: `moe-taigi-reference.md` |
| 14 | `aiongtaigi-sushi/` (decompiled) | Kotlin + Compose (decompiled) | Android (IME) | Full IME (Taigi) | Unknown (closed source; class names obfuscated `p001a0` etc.) | Unknown | Has on-device `hanji_corrections.csv` orthography map | TL primary | Closed-source | **Hanji-correction CSV** at `resources/assets/hanji_corrections.csv` — direct reference for an orthography normalisation pass (`beh,欲,卜` style mapping from preferred → deprecated form) |
| 15 | `lexical-models/` (Keyman) | TypeScript | Cross (Keyman) | Predictive model registry | Trie-based wordlist + frequency | Keyman's prediction algorithm | Not in scope | n/a (per-locale) | MIT | **Folder convention** (`release/<author>/<bcp47>.<uniq>/`) for shipping pluggable predictive models. Worth borrowing if we ever externalise dictionary distribution |
| 16 | `trime/` | Kotlin/Java + JNI (C++) | Android (IME) | RIME wrapper | Delegates to librime | librime's Spelling Algebra | librime user dict (LevelDB) | librime syllabifier (per-schema YAML) | **GPL-3.0-or-later** | **Android counterpart to Hamster** — only open-source, actively-maintained reference for embedding a native engine into an Android IME via JNI. Async `RimeDispatcher`/`RimeDaemon`/`RimeSession` pattern + 3 candidate-UI modes + on-device RIME data deployment |

---

## Topic index

If you are working on… → read these in order.

### Segmentation / lattice (DP, DAG, Viterbi)

1. **`McBopomofo/algorithm.md`** — clearest pedagogical write-up (ReadingGrid, Spans, topological-sort relaxation). Required reading before touching `engine/composing/src/walker/`.
2. **`khiin-rs/khiin/src/data/segmenter.rs`** — word-level DP cost function. Already mirrored in our roadmap Phase 9 user_freq_boost work.
3. **`librime/src/rime/algo/syllabifier.cc`** — DAG construction with prism (double-array trie). Industry baseline.
4. **`moe-taigi-reference.md`** — segment-by-segment "Nail" UX (we explicitly rejected this in 2026-03; keep for context).

### Candidate ranking (user_freq, recency, bigram)

1. **`librime` user-dict math** — `src/rime/dict/user_dictionary.cc` for `formula_d(d, t, da, ta) = d + da * exp((ta - t)/200)`. Time-decay formula reference. See `rime-reference.md`.
2. **`McBopomofo` epsilon boost** — `Source/Engine/McBopomofoLM.cpp:120`. Single-char user phrases get `topScore + 1e-9` so they don't always beat multi-syllable. Same problem we hit in Phase 9.4a.
3. **`khiin-rs` bigram** — `khiin/src/data/database.rs` SQL schema for `bigrams(lgram, rgram, n)`.
4. **`librime-predict`** — minimal next-word predictor (read all of `src/`, ~500 LOC).

### User adaptation / personal dict

1. **`librime` LevelDB user_dict** — canonical key-value (`{code}\t{phrase}` → `c=N d=D t=T`).
2. **`McBopomofo` UserPhrasesLM + ExcludedPhrases** — separate user-add vs system-exclude pipelines.
3. **`moe_taigi_apk` UserVoc vs LearnedVoc** — explicit separation between manual additions and auto-learning, with maturity thresholds (`RIPE_*_APPROVALS`).

### Syllabifier (POJ / TL / Bopomofo / TPS)

1. **`knowledge/taigi-phonetics-reference.md`** — our internal source of truth. Always first.
2. **`khiin-rs/ji/src/lomaji.rs` + `tone.rs`** — closest match for what our Rust syllabifier does.
3. **`rime-moetaigi/moetaigi.schema.yaml`** — TPS (Bopomofo-style) tone-mark placement. Reference for `keyboard/tps/*`.
4. **`McBopomofo/Source/Engine/Mandarin/`** — Bopomofo input validation. Useful as a structural mirror; phonetic rules don't transfer.

### Predictive / next-word

1. **`librime-predict/src/predictor.cc`** — full plugin, ~7 source files, easiest to read end-to-end.
2. **`lexical-models/`** — Keyman's external predictive-model registry; relevant if we ever want pluggable dictionaries.

### Native-engine embedding / FFI threading (mobile)

1. **`trime/app/src/main/java/com/osfans/trime/core/` + `daemon/`** — Android: engine on one dedicated thread, `suspend` `RimeApi` + `SharedFlow` events, lifecycle ready-gating, ref-counted sessions. Closest open-source model for our Android↔Rust FFI threading.
2. **`Hamster/`** — iOS: how to build & embed librime as an iOS framework (the iOS-side counterpart; see card #7).
3. **External: fcitx5-android** — the dispatcher/daemon pattern Trime is adapted from; clone on demand if the abstraction itself needs scrutiny.

### Custom keyboard layout / UI

1. **`azooKey/KeyboardViews/Custard/`** — declarative grid-fit layout from JSON. See `azookey-reference.md`.
2. **`CustardKit/json/howToMake.md`** — the JSON schema spec itself (Japanese-only doc).
3. **`florisboard/lib/snygg/`** — Compose-based theming DSL for IME. Closest to what we'd want for Android theming.
4. **`KeyboardKit-Documentation/`** — vendor SDK we run on; always consult before suspecting a KK bug.
5. **`trime/app/src/main/java/com/osfans/trime/ime/candidates/`** — three Android candidate-render modes (popup / compact / unrolled); `unrolled/CandidatesPagingSource` pages a large candidate list via AndroidX Paging3.

### Schema / config-driven IME

1. **`librime/`** — gold standard. YAML schemas with `__include`, `__patch`, `__append`, `__merge`. We do **not** want this level of flexibility, but the YAML shape is the reference if we externalise anything.
2. **`rime-moetaigi/moetaigi.schema.yaml`** — minimal Taigi-shaped RIME schema.
3. **`trime/app/src/main/java/com/osfans/trime/provider/RimeDataProvider.kt` + `app/data/rime/`** — how a RIME schema/dict bundle is *deployed and exposed* on Android (submodule-vendored data + `DocumentsProvider`).

### Taigi-specific UX / data

1. **`moe_taigi_apk`** — MOE official, ships on Android. Reference for what Taiwanese users see as "default".
2. **`aiongtaigi-sushi`** — `hanji_corrections.csv` (preferred → deprecated form). Reference data for orthography normalisation.
3. **`rime-moetaigi`** — 中文反查 (Mandarin reverse-lookup), 注音顯示 (show TPS next to candidate). Both features we have considered.

---

## Per-repo cards

### 1. azooKey (iOS) — `references/azooKey/`

- **What**: Japanese iOS keyboard, SwiftUI-based, ships own kana→kanji engine (`zenz-v3` neural LM in latest versions).
- **Why we care**: Best open-source iOS keyboard reference. Dual-track layout (Flick `CustardKit` + QWERTY `UnifiedKey`), 47-action protocol, SwiftUI patterns (`@StateObject` + `@EnvironmentObject` + enum-based NavigationStack routing).
- **Where to look**:
  - `KeyboardViews/View/UnifiedKey/` — unified key system
  - `KeyboardViews/Custard/` — built-in Flick layouts
  - `MainApp/Setting/` — settings UI
- **Inspiration takeaways for us**:
  - Flick five-way input → Taigi tone variations (`a → á/à/â/ā/a̍`)
  - `UnifiedKeyModelProtocol` → unified key interface
  - `MatchedGeometryEffect` for tone-mark animation (already used)
- **Deep-dive**: [`azookey-reference.md`](./azookey-reference.md)

### 2. azooKey-Desktop (macOS) — `references/azooKey-Desktop/`

- **What**: macOS port using Input Method Kit; shares the engine `Core/` package with iOS azooKey.
- **Why we care**: Reference for IMK integration if Taigi ever ships on macOS. Also shows how to package the same Swift engine as both iOS keyboard extension and macOS IMK plugin.
- **Where to look**:
  - `azooKeyMac/` — IMK entry, `InputController.swift`
  - `Core/Sources/` — shared engine
- **Inspiration takeaways**: shared-core packaging pattern (matches Taigi Phase IV-A direction). Pure Swift shared layer talks to per-platform thin wrappers.

### 3. CustardKit — `references/CustardKit/`

- **What**: Swift package + JSON schema for declaring custom keyboard tabs that azooKey can load. Companion tool, not an IME itself.
- **Why we care**: If we ever ship a "share your keyboard layout" feature, this is the existing data format to mirror.
- **Where to look**:
  - `json/howToMake.md` — schema spec (Japanese)
  - `swift/howToMake.md` — Swift API docs
  - `resource/structure.png` — keyboard structure diagram azooKey assumes
- **Status for us**: not planned (YAGNI), keep as reference only.

### 4 & 5. KeyboardKit / KeyboardKit-Documentation — `references/keyboardkit9.9.0/`, `references/KeyboardKit-Documentation/`

- **What**: The iOS keyboard SDK Taigi runs on. Open core + commercial Pro tiers.
- **Why we care**: When a bug looks like KK behaviour, *always* check the local docs clone before WebSearching. Versions drift; the doc folder is pinned to the version we use.
- **Where to look**:
  - `keyboardkit9.9.0/Sources/KeyboardKit/` — open-source modules
  - `KeyboardKit-Documentation/` — full API docs (markdown)
- **Inspiration takeaways**: framework-level patterns (autocomplete provider protocol, layout provider chain). Not "things to copy" — things to know our wrapper sits on top of.

### 6. FlorisBoard — `references/florisboard/`

- **What**: Open-source Android keyboard framework, Apache 2.0. v0.6 milestone aims to ship word suggestions; current `libnative/dummy` confirms the NLP/suggestion native code is **not yet in the OSS repo**.
- **Why we care**: Best open Android keyboard *framework* to mirror for IME service skeleton, Compose-based IME UI, snygg theming DSL, gradle multi-module setup.
- **Where to look**:
  - `app/src/main/kotlin/dev/patrickgold/florisboard/` — IME service + Compose UI
  - `lib/snygg/` — theming DSL
  - `lib/native/` — JNI bindings layout (we use a similar pattern for Rust engine)
  - `ROADMAP.md` — what they're prioritising
- **Inspiration takeaways**: Compose-based IME UI patterns (we use Compose too); subclassing `InputMethodService`; clipboard manager UX; addons store concept.
- **Caveat**: do NOT look here for **suggestion engine** logic — it isn't shipped. Use librime / khiin-rs / McBopomofo instead.

### 7. Hamster — `references/Hamster/`

- **What**: iOS keyboard that embeds librime as a framework. Originally GPL-3.0, switched to MIT at v2.1.0; subsequent "Cang" rebrand went closed-source.
- **Why we care**: Practical reference for **how to ship librime on iOS**: `librimeFramework.sh`, `LibrimeKit` GitHub Actions build, KeyboardKit integration.
- **Where to look**:
  - `librimeFramework.sh` + `InputSchemaBuild.sh` — build pipeline
  - `HamsterKeyboard/` — KeyboardKit-based extension target
  - `Hamster/` — main app
- **Inspiration takeaways**: If Taigi ever wants to *embed* librime instead of building our own engine (Phase IV-A made that explicitly not our path), this is the prior art. Useful as a "we considered but rejected" cite.

### 8. librime — `references/librime/`

- **What**: The canonical RIME engine. C++. Powers Squirrel (macOS), Weasel (Windows), ibus-rime (Linux), fcitx5-rime, Hamster, RIME 鼠鬚管 (iOS), and dozens of derivatives.
- **Why we care**: This is the **architectural reference** every other modular IME is measured against. Modular Pipeline (Processor → Segmentor → Translator → Filter), DAG segmentation, Spelling Algebra, dynamic-weight + time-decay user dict.
- **Where to look**:
  - `src/rime/algo/` — `dynamics.cc`, `syllabifier.cc`, `algebra.cc`
  - `src/rime/dict/` — `user_dictionary.cc`, `prism.cc`, `table.cc`
  - `src/rime/gear/` — processors, translators, filters
  - `src/rime/config/` — `config_compiler.cc`, `config_types.cc`
- **Inspiration takeaways**:
  - **Time-decay weight formula** (`formula_d(d, t, da, ta) = d + da * exp((ta - t)/200)`) directly informed our recency_rank work.
  - **Penalty values** as design vocabulary: -0.693 (= ln 2) for fuzzy/abbrev/autocomplete, -4.605 (= ln 100) for error correction, -23.026 (= ln 1e10) for ambiguity.
- **Deep-dive**: [`rime-reference.md`](./rime-reference.md)

### 9. librime-predict — `references/librime-predict/`

- **What**: Tiny librime plugin (~7 source files) that adds next-word prediction via a separate `predict.db` table.
- **Why we care**: Smallest readable reference for "add a predictor without rewriting the engine". Demonstrates a clean plugin interface.
- **Where to look**:
  - `src/predict_db.{h,cc}` — schema
  - `src/predict_engine.cc` — prediction logic
  - `src/predict_translator.cc` — RIME translator integration
  - `README.md` — full config + integration example
- **Inspiration takeaways**: Config knobs (`max_candidates`, `max_iterations`) for predictor budget. We may want similar guards if next-word ever becomes user-tunable.

### 10. rime-moetaigi — `references/rime-moetaigi/`

- **What**: The only mainstream Taigi schema for RIME. Built on the MOE 《臺灣閩南語常用詞辭典》 corpus (~20k entries) by Whyjay Zheng. CC0.
- **Why we care**: Reference data for **TPS tone-mark placement**, **中文反查** (Mandarin reverse-lookup) mode, **注音顯示** (TPS-next-to-candidate) mode, and **shift-modified Bopomofo keys** for Taigi-only phonemes (ㄫ, etc.).
- **Where to look**:
  - `moetaigi-tsuim.schema.yaml` — main schema
  - `moetaigi.dict.yaml` + `moetaigi.extended.dict.yaml` — dict entries
  - `moetaigi.unspaced.schema.yaml` — version that allows continuous input without spaces
  - `doc/images/Keyboard_layout_Tsuim.png` — TPS keyboard layout reference
- **Inspiration takeaways**:
  - Keyboard layout for TPS (already in our `keyboard/tps/*`)
  - 中文反查 mode UX (we have this as a roadmap consideration)
  - Auto user-phrase via librime — *not* our path (we ship custom Rust engine), but good "how the other side does it" reference.

### 11. McBopomofo — `references/McBopomofo/`

- **What**: Open-source macOS Mandarin IME, IMK-based, Bopomofo. Has the **best algorithm documentation** of any IME we've examined (`algorithm.md`).
- **Why we care**:
  - **Gramambular DAG** (`Source/Engine/gramambular2/reading_grid.{h,cpp}`) is the cleanest reference implementation of unigram + topological-sort + relaxation walker. Maps almost 1:1 onto our Phase 9 lattice work.
  - **Heterophone handling** (`Source/Data/curation/compilers/main_compiler.py:158`): first reading = original freq, second reading × 0.5, third × 0.25, others = floor `-6.8`. Directly applicable to TL heterophones.
  - **User-phrase epsilon boost** (`McBopomofoLM.cpp:120`): single-syllable user phrases get `topScore + 1e-9` — solves the exact "user word always wins over multi-syllable phrase" bug we hit in v3.5.6.
  - **`fscale = 2.7`** length weighting in `frequency_builder.py` — empirical multiplier for "long words deserve a boost". Reference for any future Taigi DP cost-function tuning.
- **Where to look**:
  - `algorithm.md` — MANDATORY READ before touching `engine/composing/src/walker/`
  - `Source/Engine/gramambular2/reading_grid.cpp:51 (insertReading), :134 (Relax), :166 (TopologicalSort), :216 (walk), :417 (update)`
  - `Source/Engine/McBopomofoLM.cpp:81 (getUnigrams), :120 (epsilon boost), :234 (filterAndTransformUnigrams)`
  - `Source/Engine/ParselessLM.{h,cc}` + `ParselessPhraseDB.{h,cc}` — memory-mapped binary-search dict (alternative to MARISA-trie)
  - `Source/Data/curation/` — Python pipeline for building dict from raw corpora
- **Inspiration takeaways for us**: too many to list; this is **the** reference for our lattice work. Phase 9.4a/9.4b/9.4c all touch ideas pioneered or cleanly documented here.

### 12. khiin-rs — `references/khiin-rs/`

- **What**: Rust-based Taigi IME for desktop (Win/Linux). Closest open-source architectural cousin to Taigi Keyboard.
- **Why we care**: Three input modes (Continuous / Classic / Manual), POJ/TL syllable validation, Telex tone input (`s/f/l/j`), unigram + bigram DB, word-level DP segmentation.
- **Where to look**:
  - `ji/src/lomaji.rs` + `tone.rs` — tone conversion, syllable validation
  - `khiin/src/data/segmenter.rs` — DP segmentation cost function
  - `khiin/src/input/converter.rs` — candidate generation
  - `khiin/src/buffer/buffer_mgr.rs` — buffer state machine
- **Inspiration takeaways**:
  - **Three input modes** — already mirrored in our keyboard (Phase 8).
  - **Cost function** `ln(1/p^FREQ_BIAS) / word_len^LET_BIAS * n_syls^SYL_BIAS` — reference for our Phase 9 ranking.
  - **Two-stage architecture** — segmenter (pure function) + selector (dict-aware) is the model we chose.
  - **Why we don't fully mirror it**: khiin targets desktop sentence input (10+ words); mobile keyboard does 1-3 words with candidate bar. Our `len^2` + dict-tie-break is cheaper and sufficient.
- **Deep-dive**: [`khiin-reference.md`](./khiin-reference.md)

### 13. moe_taigi_apk (decompiled) — `references/moe_taigi_apk/`

- **What**: MOE's official Taigi Android IME, decompiled from APK. C++ engine via SWIG JNI. Closed source.
- **Why we care**: **Reference UX** for Taiwanese users — the keyboard most teachers/learners default to.
- **Where to look**:
  - `decompiled/sources/moe/taigi/Tailo.java` — SWIG wrapper
  - `decompiled/sources/moe/taigi/TailoJNI.java` — JNI method declarations (full API surface)
  - `decompiled/sources/android/.../CandidateModel.java` — candidate shape (`spanUnits`, `tailos: List<String>`)
  - `decompiled/sources/android/.../KeySectionsModel.java` — `composedCharacters` + `composingCharacters` split
  - `extracted/assets/tailo.tab` — binary Trie database
- **Inspiration takeaways**:
  - **Multiple readings per candidate** (`List<String> tailos`) — better than single TL string. We do this.
  - **Han-lo mixed candidates** (`VT_MIXED`) — show 漢字 + TL in the same candidate. We do this.
  - **UserVoc / LearnedVoc separation** — manual vs auto-learning. We don't (yet); flagged as potential v3.6 work.
  - **Nail commit flow** — segment-by-segment confirmation. **Explicitly rejected** for our UX in 2026-03; keep for context.
- **Deep-dive**: [`moe-taigi-reference.md`](./moe-taigi-reference.md) (engine analysis) + [`moe-taigi-asr-reference.md`](./moe-taigi-asr-reference.md) (their ASR sidecar)

### 14. aiongtaigi-sushi (decompiled) — `references/aiongtaigi-sushi/`

- **What**: "Aiong Taigi 阿勇台語" Android IME, Compose-based, decompiled from APK. Closed source; class names obfuscated (`p001a0/`, `p002a1/`, …).
- **Why we care**: The engine is unreadable, but `resources/assets/hanji_corrections.csv` is a goldmine — explicit `tailo,preferred_hanji,deprecated_hanji` rows like `beh,欲,卜`, `mài,莫,勿`, `siáⁿ,啥,省`. This is **prior art for an orthography normalisation pass** on top of our existing dict.
- **Where to look**:
  - `resources/assets/hanji_corrections.csv` — orthography map (read all of it)
  - `resources/assets/composeResources/sushi_ime.compose.generated.resources/` — Compose resource bundle
  - `resources/AndroidManifest.xml` — IME service entry
- **Inspiration takeaways**:
  - **Hanji orthography normalisation** as a separable layer (preferred → deprecated mapping). Distinct from frequency tuning.
  - Compose-based Taigi IME on Android is feasible (we already mirror via FlorisBoard's Compose IME).
- **Caveat**: do NOT try to read the obfuscated Kotlin classes — wasted token budget. The CSV is the only useful artefact.

### 15. lexical-models (Keyman) — `references/lexical-models/`

- **What**: Keyman Foundation's open-source predictive-model registry. TypeScript build pipeline that compiles wordlists into `.model.kmp` packages keyed by BCP-47 language tags.
- **Why we care**: **Folder convention** for shipping pluggable predictive dictionaries: `release/<author>/<bcp47>[.<uniq>]/`. If Taigi ever externalises dictionary distribution (so users can add custom wordlists per-locale), this is the existing format to consider mirroring.
- **Where to look**:
  - `release/` — actual shipped models (varied: `en.*`, `km.*`, …)
  - `experimental/` — work-in-progress models
  - `tools/` — compiler scripts
  - `docs/externally-hosted-models.md` — how third parties publish their own
- **Inspiration takeaways**: out-of-tree dictionary distribution pattern. Probably YAGNI for us until v4.x.

### 16. Trime (Android) — `references/trime/`

- **What**: `osfans/trime` — "Trime / 同文输入法", the **RIME IME for Android**. Kotlin/Java front-end + JNI bridge over `librime` (vendored as a submodule, alongside OpenCC / librime-predict / librime-octagram / librime-lua). ~25k LOC Kotlin/Java + ~1.3k LOC JNI C++. Nightly checkout `277b8ea2` (2026-05-17 clone). License **GPL-3.0-or-later** — the most restrictive among our refs; read for **architecture only**, never copy code.
- **Why we care**: This is the **Android counterpart to Hamster** (which embeds librime on iOS). Hamster + Trime together are the two source-available references for "wrap a native engine in a mobile IME". Trime fills two gaps in our corpus: `florisboard` (our Android architecture mirror) has its NLP module **gutted**, and `moe_taigi_apk` (Android Taigi IME) is **decompiled/closed** — Trime is a full, readable Android IME *with a real engine wired in via a clean async daemon*. Directly relevant to how our Android side calls the **Rust** engine over FFI.
- **Where to look**:
  - `app/src/main/java/com/osfans/trime/core/` — JNI surface as Kotlin: `RimeApi` (suspend interface), `RimeDispatcher` (single-threaded executor exposed as a `CoroutineDispatcher`), `RimeLifecycle` (ready-gating), `RimeMessage`/`messageFlow: SharedFlow` (engine→UI events). `RimeDispatcher`/`RimeDaemon` are explicitly *adapted from fcitx5-android* (see External pointers → fcitx5).
  - `app/src/main/java/com/osfans/trime/daemon/` — `RimeDaemon` (singleton engine, ref-counted `RimeSession`s, `Rime.finalize()` when no clients) + `RimeSession` (`run` / `runOnReady` / `runIfReady` ready-state contract).
  - `app/src/main/java/com/osfans/trime/ime/candidates/` — three render modes: `popup/`, `compact/`, `unrolled/` (the last paged via AndroidX **Paging3** `CandidatesPagingSource` over `getCandidates(offset, limit)`).
  - `app/src/main/jni/librime_jni/` — the FFI marshalling layer (`rime_jni.cc`, `session.h`, `frontend.cc`, `levers.cc`; ~1.3k LOC).
  - `app/src/main/java/com/osfans/trime/provider/RimeDataProvider.kt` — `DocumentsProvider` exposing the on-device RIME user dir; schema data vendored via git submodules under `app/data/rime/`.
- **Inspiration takeaways for us**:
  - **Async engine-daemon pattern**: marshal every FFI call onto one dedicated engine thread; expose a `suspend` API + a `SharedFlow` of engine events; gate calls behind a lifecycle "ready" state; ref-count sessions so the engine finalizes when no IME/host is attached. This is the model for our Android↔Rust FFI threading.
  - **Paged candidate UI**: `CandidatesPagingSource` shows how to lazily page a large candidate list out of the engine instead of materialising it all — useful if our Android candidate bar ever needs an "unrolled" full-list view.
  - **Engine data deployment on Android**: submodule-vendored schema/dict + a `DocumentsProvider` for user access — reference if we ever let users inspect/edit on-device dictionary files.
- **Caveat**: librime engine internals (segmentation / Spelling Algebra / user-dict math) are **already covered** by `librime` (#8) + `rime-reference.md`; do not re-derive them from Trime. Trime's value is the **Android integration layer**, not the engine.

---

## External pointers (NOT cloned under `references/`)

Worth knowing about; clone on demand when a specific question arises.

### Mozc — https://github.com/google/mozc

- Google Japanese Input, OSS C++. The **reference implementation** for n-gram + lattice + Viterbi in Japanese IME. Most modern Japanese IMEs (azooKey included) trace lineage to Mozc's converter design.
- Read when: designing a new lattice algorithm; choosing between unigram-only vs bigram lattice; questioning whether to add proper Viterbi vs the topological-sort relaxation McBopomofo uses.

### libchewing — https://github.com/chewing/libchewing

- Mature OSS Bopomofo engine (Hakka/Mandarin), C. The IBM/Linux Bopomofo IME most distros ship with. SQLite-backed user dict, smart-phrase autocomplete.
- Read when: comparing alternative storage layouts for user freq (libchewing's SQLite schema is simpler than RIME's LevelDB).

### Gboard (Google) — proprietary, but see research papers

- "Federated Learning of Out-Of-Vocabulary Words" (Hard et al. 2018) and "Federated Learning for Mobile Keyboard Prediction" (McMahan et al. 2018) are the canonical references for on-device personalisation without server uploads.
- Read when: considering privacy-preserving user-freq sharing across devices (Phase IV+ stretch goal).

### OpenVanilla — https://github.com/openvanilla/

- Umbrella project, McBopomofo's home. Hosts McBopomofo, OpenVanilla framework (older), and several utility IMEs.
- McBopomofo (already covered above) is the most-maintained module.

### fcitx5 / ibus — input method *frameworks*, not engines

- Both are framework hosts; engines (rime, mozc, libchewing, anthy) plug into them.
- Read when: questioning Linux integration patterns. Not relevant for iOS/Android-focused Taigi Keyboard.

---

## Cross-cutting insights (Taigi-specific reading order)

When designing a Phase IV+ engine slice, read in this order:

1. **`knowledge/taigi-phonetics-reference.md`** — what TL/POJ/TPS *should* do.
2. **`docs/engine/*.md`** — what our current engine does.
3. **`khiin-reference.md`** (deep-dive) — closest OSS cousin; what they tried and what they tuned.
4. **`McBopomofo/algorithm.md`** — best documented lattice algorithm; transfer the algorithm shape, not the phonetic rules.
5. **`rime-reference.md`** (deep-dive) — pipeline modularity + user-dict math; treat as design vocabulary, not as code-to-copy.
6. **`moe-taigi-reference.md`** (deep-dive) — what Taiwanese users currently expect as default IME UX.
7. **This file** (mainstream-ime-comparison.md) — back-pointer when you forget which repo covers which dimension.

For Phase II+ (cross-platform alignment), read:

1. **`azookey-reference.md`** — iOS UI patterns.
2. **`florisboard/app/src/main/kotlin/dev/patrickgold/florisboard/`** — Android IME service skeleton + Compose IME UI (engine *not* wired in).
3. **`trime/app/src/main/java/com/osfans/trime/core/` + `daemon/`** — Android IME *with* a native engine wired in via an async daemon; the FFI-threading model for our Android↔Rust boundary.
4. **`.claude/rules/cross-platform-alignment.md`** — refactor-freeze + Phase II end gate.

---

## Maintenance log

- **2026-05-12** — Initial version. Indexed 13 repos under `references/` + 4 external pointers (Mozc, libchewing, Gboard, OpenVanilla). Built on top of existing deep-dives (azookey/khiin/rime/moe-taigi).
- **2026-05-17** — Added repo #16 `trime/` (osfans/trime, RIME IME for Android, GPL-3.0, nightly `277b8ea2`). Matrix row + per-repo card + new "Native-engine embedding / FFI threading" topic section + Custom-UI / Schema-deploy / Phase II+ pointers. Index-only (no deep-dive): Trime's engine internals are already covered by `librime` (#8) + `rime-reference.md`; its value is the Android integration layer. Deep-dive deferred until an Android FFI/daemon slice needs it.

When adding a new repo under `references/`, append a card here and a row in the TL;DR matrix; if the repo is deep enough to warrant its own deep-dive (>500 LOC of read-through), create `docs/references/<repo>-reference.md` and link both ways.
