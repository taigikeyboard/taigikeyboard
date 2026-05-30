# Mainstream IME Comparison

> **Type**: Reference index
> **Purpose**: Centralised cross-reference of every mainstream IME / keyboard repo cloned under `references/`, plus a few external projects worth knowing. Read this **before** writing a `最佳實踐對齊` section in a plan, before designing a new engine slice, or before asserting "Project X already does Y".
> **Status**: Authoritative as of 2026-05-30. Update when adding a new repo under `references/` or when an existing deep-dive doc lands in `docs/references/`.
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
| 17 | `mozc/` | C++ (Bazel) | Cross (Android lib / macOS / Win / Linux / ChromeOS) | Full IME (Japanese, Google) | Connector-cost **lattice + Viterbi** (`converter/lattice.cc`, `immutable_converter.cc`, `nbest_generator.cc`) | Connection-matrix + POS cost + segment rerank (`rewriter/`) | `dictionary/user_dictionary.cc` + storage + suppression dict | Kana composer (`composer/`) | BSD-3-Clause | **Canonical n-gram + lattice + Viterbi reference** that modern Japanese IMEs (azooKey incl.) trace lineage to. Promoted from external pointer → cloned `afbf1d089` (2026-05-27) |
| 18 | `rakukan/` | Rust | Windows (TSF) | Full IME (Japanese, LLM) | LLM (llama.cpp `jinen`) + mozc/SKK dict merge; live-conversion + range-select flow | Dict-merge + user-dict learning + literal (digit/alpha) protection | `rakukan-dict` user dict; immediate commit learning | Kana/romaji (`engine/kana.rs`, `romaji/`) | MIT | **Rust IME with out-of-process engine-host** (`rakukan-engine-host.exe` over Named-Pipe + postcard RPC) + **live conversion**. Closest arch analogue for our Rust-engine FFI + continuous input. `ea8e151` (2026-05-29) |
| 19 | `PIME/` | C++ + Python/Node/Go | Windows (TSF) | IME framework (multi-backend host) | per-backend (hosts libchewing, McBopomofoWeb, …) | per-backend | per-backend | per-backend | LGPL-2.1 (mixed; per-part) | **Out-of-process multi-backend IME host**: TSF C++ shell ↔ python/node/go server over JSON (`backends.json`). Reference for hosting a foreign engine (e.g. McBopomofoWeb node backend) behind one Windows IME shell. `571759f` (2026-05-16) |
| 20 | `MacishType/` | Swift + JavaScript | macOS (IMK) | IME shell (JS-pluggable) | engine-defined (external JS) | engine-defined | engine-defined | engine-defined | MIT | **Native-look candidate window** (horizontal / vertical / expandable, app-accent-color match) + **JS-pluggable engine** with `manifest.json`-generated Settings UI. Reference for candidate-window layouts + config-driven engine plugin. `8a6cb3a` (2026-05-27) |

---

## Topic index

If you are working on… → read these in order.

### Segmentation / lattice (DP, DAG, Viterbi)

1. **`McBopomofo/algorithm.md`** — clearest pedagogical write-up (ReadingGrid, Spans, topological-sort relaxation). Required reading before touching `engine/composing/src/walker/`.
2. **`khiin-rs/khiin/src/data/segmenter.rs`** — word-level DP cost function. Already mirrored in our roadmap Phase 9 user_freq_boost work.
3. **`librime/src/rime/algo/syllabifier.cc`** — DAG construction with prism (double-array trie). Industry baseline.
4. **`mozc/src/converter/immutable_converter.cc` + `lattice.cc` + `nbest_generator.cc`** — full connection-cost **Viterbi** + N-best. The "do it properly" end of the spectrum; read when justifying whether a Taigi slice needs a real connection matrix or the cheaper topological-sort relaxation (McBopomofo) / our walker suffices.
5. **`moe-taigi-reference.md`** — segment-by-segment "Nail" UX (we explicitly rejected this in 2026-03; keep for context).

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
3. **`mozc/src/prediction/dictionary_predictor.cc` + `dictionary_prediction_aggregator.cc`** — production-grade suggestion aggregation (dictionary + zero-query). Read when our prediction needs to merge multiple candidate sources with budget caps.
4. **`rakukan/docs/LIVE_CONV_REDESIGN_REVISED.md`** + `crates/rakukan-engine/src/segments.rs` — **live conversion** (auto-show top candidate on pause) + literal protection. Closest non-Taigi reference for our continuous-input candidate flow.

### Native-engine embedding / FFI threading (mobile)

1. **`trime/app/src/main/java/com/osfans/trime/core/` + `daemon/`** — Android: engine on one dedicated thread, `suspend` `RimeApi` + `SharedFlow` events, lifecycle ready-gating, ref-counted sessions. Closest open-source model for our Android↔Rust FFI threading.
2. **`Hamster/`** — iOS: how to build & embed librime as an iOS framework (the iOS-side counterpart; see card #7).
3. **External: fcitx5-android** — the dispatcher/daemon pattern Trime is adapted from; clone on demand if the abstraction itself needs scrutiny.
4. **`rakukan/` out-of-process host** — `crates/rakukan-engine-host/` + `rakukan-engine-rpc/` (Named-Pipe + postcard) + `rakukan-engine-abi/` (DLL loader). Desktop answer to engine isolation: the heavy Rust/LLM/GPU engine runs in a **separate process** behind a thin RPC. Read when engine crashes / GPU memory ever push us toward sandboxing the engine off the keyboard surface (#18).
5. **`PIME/` multi-backend host** — `backends.json` + `PIMETextService/` + `PIMELauncher/`. One TSF shell ↔ N engines (python/node/go) over JSON IPC; runs McBopomofoWeb as a node backend. The "one shell, many engines, stable IPC boundary" concept (#19). Transport (process fork + IPC) does **not** transfer to a sandboxed mobile keyboard — only the shell/engine separation does.

### Custom keyboard layout / UI

1. **`azooKey/KeyboardViews/Custard/`** — declarative grid-fit layout from JSON. See `azookey-reference.md`.
2. **`CustardKit/json/howToMake.md`** — the JSON schema spec itself (Japanese-only doc).
3. **`florisboard/lib/snygg/`** — Compose-based theming DSL for IME. Closest to what we'd want for Android theming.
4. **`KeyboardKit-Documentation/`** — vendor SDK we run on; always consult before suspecting a KK bug.
5. **`trime/app/src/main/java/com/osfans/trime/ime/candidates/`** — three Android candidate-render modes (popup / compact / unrolled); `unrolled/CandidatesPagingSource` pages a large candidate list via AndroidX Paging3.
6. **`MacishType/MacishCandidateWindow/` + `Engines/README.md`** — macOS native candidate-window UX: horizontal / vertical / **expandable** layouts, paging-vs-expand, app-accent-color match, `pageSize` / `fontSize` / index-label knobs. The README `candidateWindow` table is a tidy checklist of candidate-UI options (#20).

### Schema / config-driven IME

1. **`librime/`** — gold standard. YAML schemas with `__include`, `__patch`, `__append`, `__merge`. We do **not** want this level of flexibility, but the YAML shape is the reference if we externalise anything.
2. **`rime-moetaigi/moetaigi.schema.yaml`** — minimal Taigi-shaped RIME schema.
3. **`trime/app/src/main/java/com/osfans/trime/provider/RimeDataProvider.kt` + `app/data/rime/`** — how a RIME schema/dict bundle is *deployed and exposed* on Android (submodule-vendored data + `DocumentsProvider`).
4. **`PIME/backends.json`** — minimal registry mapping `name → command/workingDir/params` for pluggable engine backends. The whole multi-engine dispatch in ~20 lines.
5. **`MacishType/Engines/README.md`** — `manifest.json` engine contract: declare a field → it's fixed; omit it → host auto-exposes a user control. Clean "config sets it OR user controls it" model for settings-vs-defaults seams.

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

### 17. mozc (Google Japanese Input) — `references/mozc/`

- **What**: Google's OSS Japanese IME, C++ / Bazel. Multi-platform (Android lib, macOS, Windows, Linux, ChromeOS). Origin of Google Japanese Input. BSD-3-Clause. Clone `afbf1d089` (2026-05-27). *Previously an external pointer; now cloned — see card here instead of the bottom section.*
- **Why we care**: The **canonical n-gram + lattice + Viterbi** Japanese converter. Most modern Japanese IMEs (azooKey included) trace their converter design to Mozc. Read it when comparing **full Viterbi** (Mozc) vs the **topological-sort relaxation** McBopomofo uses (#11) vs our walker — i.e. when deciding whether a Taigi lattice slice needs a real connection-cost matrix or the cheaper relaxation is sufficient.
- **Where to look**:
  - `src/converter/lattice.{h,cc}` — the lattice node structure
  - `src/converter/immutable_converter.{h,cc}` — Viterbi over the lattice with connection costs
  - `src/converter/nbest_generator.{h,cc}` — N-best extraction (A* over the back-pointers)
  - `src/converter/segments.{h,cc}` — segment / candidate container
  - `src/prediction/dictionary_predictor.cc` + `dictionary_prediction_aggregator.cc` — prediction / suggestion
  - `src/dictionary/user_dictionary.cc` + `user_dictionary_storage.cc` — user dict + import + suppression dictionary
  - `src/rewriter/` — post-conversion candidate rerankers (date/number/symbol/usage)
  - `src/composer/` — kana composition (romaji→kana table-driven)
- **Inspiration takeaways for us**:
  - **Connection-cost matrix + Viterbi** is the "do it properly" end of the lattice spectrum; our walker + McBopomofo's relaxation sit at the cheaper end. Cite Mozc when justifying why we *don't* need a full bigram connection matrix on mobile.
  - **Suppression dictionary** (`user_dictionary` storage) — a clean "never surface this entry" layer separate from frequency. Reference if we ever add a user-level blocklist.
  - **Rewriter pipeline** — post-Viterbi candidate transforms (number formatting, date) as composable passes; structurally similar to where our display-dedupe / orthography passes could live.
- **Caveat**: Japanese-specific (kana, bunsetsu segmentation). Transfer the **algorithm shape**, not the linguistic rules — same rule as McBopomofo.

### 18. rakukan — `references/rakukan/`

- **What**: `rakukan` v0.9.3 — experimental **Rust** Windows Japanese IME. Core conversion = small local **LLM** (llama.cpp `jinen` model) + **mozc / SKK dictionaries** merged. TSF integration; GPU (CUDA / Vulkan) backends. MIT. Clone `ea8e151` (2026-05-29). Builds on `karukan`'s LLM converter + `azooKey-Windows` TSF layer.
- **Why we care**: Architecturally the **closest non-Taigi cousin to our own engine** on two axes:
  1. **Rust engine behind a thin platform shell** — exactly our Rust-engine + iOS/Android-wrapper split, here as `rakukan-tsf` (DLL shell) ↔ `rakukan-engine` (conversion DLL).
  2. **Out-of-process engine isolation** — the engine runs in a separate **`rakukan-engine-host.exe`** process, reached over **Named-Pipe + `postcard` RPC** (`rakukan-engine-rpc`), with an ABI loader (`rakukan-engine-abi`) swapping CPU/Vulkan/CUDA engine DLLs. The IME shell stays light + stable while heavy LLM/GPU work is sandboxed. Reference for "isolate the heavy engine from the keyboard surface".
  3. **Live conversion** (ライブ変換) — auto-show top candidate after a short pause; range-select (`Shift+←/→` → `Space` → `Enter`); punctuation-block splitting; **literal protection** (LLM must not mutate digits/alphanumerics: `2024ねん → 2024年`). Directly comparable to our continuous-input candidate flow.
- **Where to look**:
  - `docs/DESIGN.md` — full crate-by-crate design (Japanese); crate map at §2
  - `crates/rakukan-engine/src/` — `backend.rs`, `segments.rs`, `conv_cache.rs`, `ffi.rs`, `kana.rs`, `romaji/`, `kanji/`, `dict/`
  - `crates/rakukan-engine-host/` + `rakukan-engine-rpc/` + `rakukan-engine-abi/` — the out-of-process host + RPC + DLL-loader split
  - `crates/rakukan-tsf/` — the TSF (Windows IME) shell
  - `crates/rakukan-dict/` + `rakukan-dict-builder/` — mozc/SKK/user dict library + build tool
  - `docs/LIVE_CONV_REDESIGN_REVISED.md`, `docs/CONVERTER_REDESIGN.md` — live-conversion + converter design notes
- **Inspiration takeaways for us**:
  - **Out-of-process engine** is the desktop answer to the same isolation problem our mobile FFI solves in-process; cite as prior art if engine crashes/GPU memory ever push us toward a sandboxed engine process.
  - **Literal protection** (don't let the converter rewrite digits/alpha) is a concrete invariant worth mirroring for our continuous path.
  - **LLM-based conversion** is explicitly *not* our path (on-device dict + lattice), but rakukan is the cleanest "we considered LLM conversion" cite.
- **Caveat**: Windows/TSF + GPU/LLM specifics don't transfer to a mobile keyboard extension; read the **crate boundaries + RPC/host split + live-conv invariants**, not the Win32/CUDA glue.

### 19. PIME — `references/PIME/`

- **What**: `EasyIME/PIME` — a Windows **IME framework** that lets you implement input methods over Text Services Framework (TSF) without writing TSF C++. A C++ TSF shell (`PIMETextService` + `libIME2`) talks to a **backend server** (python / node / go) over a JSON protocol. Ships libchewing + McBopomofoWeb (node) backends. License mixed (core `libIME` LGPL-2.1). Clone `571759f` (2026-05-16).
- **Why we care**: The **multi-backend out-of-process host** pattern: one IME shell registered with the OS, N pluggable engines behind a stable JSON IPC (`backends.json` maps `name → command/workingDir/params`). It is the Windows analogue of "host a foreign engine behind one keyboard surface" — e.g. it runs **McBopomofoWeb** (a JS port of McBopomofo) as a node backend. Reference when reasoning about engine-as-separate-process + a thin protocol boundary (compare: Trime's in-process JNI daemon #16, rakukan's Named-Pipe host #18).
- **Where to look**:
  - `backends.json` — the backend registry (the whole pattern in ~20 lines)
  - `PIMETextService/` — the C++ TSF shell that dispatches to backends
  - `libIME2/` — the reusable TSF wrapper (LGPL-2.1)
  - `python/server.py` + `node/server.js` — backend server skeletons
  - `McBopomofoWeb/` — a real engine (JS McBopomofo) wired in as a backend
  - `PIMELauncher/` — process manager that spawns/monitors backend servers
- **Inspiration takeaways for us**:
  - **Stable IPC boundary between OS-facing shell and engine** — same idea as our FFI boundary, expressed as JSON-over-IPC instead of a C ABI. Useful framing when arguing where the engine/platform line should sit.
  - **One shell, many engines** — if Taigi ever hosts multiple input engines behind one keyboard, this is the registry shape.
- **Caveat**: Windows-only, IPC-not-FFI. Mobile keyboard extensions can't fork backend processes (sandbox), so the *transport* doesn't transfer — only the **shell/engine separation** concept does.

### 20. MacishType — `references/MacishType/`

- **What**: `MacishType` — a macOS Input Method Kit (IMK) IME that replicates the system candidate-window look and is **extensible via external JavaScript engines**. Swift host + JS engine sandbox. MIT. Clone `8a6cb3a` (2026-05-27).
- **Why we care**: Two clean, directly-relevant references:
  1. **Native candidate-window layouts** — horizontal / vertical / **expandable** panels, paging vs expand-to-reveal, app-accent-color matching, configurable `pageSize` / `fontSize` / index labels. The `Engines/README.md` `candidateWindow` table is a tidy spec of candidate-UI knobs.
  2. **Config-driven engine plugin** — a `manifest.json` declares the entry module, candidate-window overrides, and a **Settings UI auto-generated from the manifest** (declare a field → it's fixed; omit it → the host exposes a user control). Hot-reloaded via FSEvents + security-scoped bookmark.
- **Where to look**:
  - `MacishType/CandidateWindow.swift` + `MacishCandidateWindow/` — the candidate window rendering
  - `MacishType/InputController.swift` + `InputEngine.swift` — IMK entry + engine interface
  - `MacishType/JavaScriptEngine/` — the JS host/sandbox
  - `Engines/README.md` — the engine contract (`manifest.json` schema + candidate-window field table + Settings-UI generation rules)
  - `Engines/ExampleEngine/` — a minimal working JS engine
- **Inspiration takeaways for us**:
  - **Candidate-window knob vocabulary** (layout direction, page size, expandable rows, index labels) — a checklist when we revisit our candidate bar / overlay options.
  - **Manifest-declares-or-exposes** pattern — a clean rule for "config sets it OR user controls it"; reusable framing for our settings-vs-defaults seams.
- **Caveat**: macOS IMK + a JS plugin runtime; we ship neither. Mine the **candidate-window UX spec** and the **config/settings model**, not the IMK or JS-host code.

### Cloned but out-of-engine-scope (not IME engines)

Two repos under `references/` are **not IME engines** and are intentionally absent from the matrix/cards above. Listed here so a future session does not re-explore them looking for engine patterns:

- **`ISEmojiView/`** (`fec2d03`, 2025-11-27, MIT) — an iOS **emoji-keyboard UI component** (categories, skin-tone variants, recently-used, system-style bottom bar). Not an IME. Relevance: **UI-only** reference for our emoji palette (`EmojiPaletteView`), if we revisit emoji-picker layout. `Sources/ISEmojiView/`.
- **`KeSi/`** (`826e787`, 2025-12-16, MIT) — `i3thuan5/KeSi`, a **Tâi-bûn NLP toolkit** (Python) by 意傳科技: 斷詞, 輕聲標註, Unicode NFC + 教育部造字碼 normalisation, 漢羅↔全羅. Not an IME. Relevance: **phonetics / dictionary** tooling (see `memory/project_kesi_deprecated.md`), not the engine comparison — read `knowledge/taigi-phonetics-reference.md` + `taigi-converter/` first per CLAUDE.md Core Principle #3.

---

## External pointers (NOT cloned under `references/`)

Worth knowing about; clone on demand when a specific question arises.

### Mozc — now cloned → see card #17

- **Promoted to a cloned repo** under `references/mozc/` (2026-05-27). See **per-repo card #17** + matrix row 17 above; this entry is kept only as a redirect.

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
- **2026-05-30** — Indexed 4 newly-cloned IMEs as cards/rows #17–20: **mozc** (`afbf1d089`, BSD-3, promoted from external pointer → cloned; Mozc external entry now a redirect), **rakukan** (`ea8e151`, MIT, Rust + LLM Windows IME, out-of-process engine-host + live conversion), **PIME** (`571759f`, LGPL-2.1, Windows TSF multi-backend host), **MacishType** (`8a6cb3a`, MIT, macOS IMK, JS-pluggable engine + native candidate window). Topic-index additions: lattice (mozc Viterbi), predictive (mozc predictor + rakukan live-conv), native-engine FFI (rakukan/PIME out-of-process), candidate UI (MacishType), config-driven (PIME `backends.json` + MacishType `manifest.json`). Added a "Cloned but out-of-engine-scope" note for `ISEmojiView/` (iOS emoji UI lib, not an IME) + `KeSi/` (Tâi-bûn NLP toolkit, not an IME) so they aren't re-explored as engine refs. Index-only (no deep-dives): create `docs/references/<repo>-reference.md` if a slice ever needs >500 LOC read-through of mozc's Viterbi or rakukan's RPC/live-conv.

When adding a new repo under `references/`, append a card here and a row in the TL;DR matrix; if the repo is deep enough to warrant its own deep-dive (>500 LOC of read-through), create `docs/references/<repo>-reference.md` and link both ways.
