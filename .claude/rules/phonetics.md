---
paths:
  - "engine/phonetics/**"
  - "engine/composing/**"
  - "engine/lexicon/**"
  - "engine/protos/**"
  - "knowledge/taigi-phonetics-reference.md"
  - "taigi-converter/**"
  - "dictionary/**"
  - "ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+Phonetics.swift"
  - "ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+CaseTransform.swift"
  - "android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/ExternalLookupURLBuilder.kt"
  - "android/app/src/main/java/com/siansiansu/taigikeyboard/engine/RustEngineBridge.kt"
---

# Phonetics Rules

Mandatory reading before ANY TL / POJ / TPS schema work, FST key-family design, column design, encoding choice, canonical-form decision, or "is X dead code" audit of phonetic tables.

Project `CLAUDE.md` Core Principle #3 ("Phonetics = authoritative-source-only") binds this.

## Word identity = (漢字, 羅馬字) pair (Core Principle #7)

A Taiwanese word is identified by the **(Hanji, canonical-TL) combination**, never by either field alone — 一字多音 (`重/tîng` ≠ `重/tāng`) and homophones make both fields necessary. Any key / dedup / group / lookup / accent-or-variant substitution over Taiwanese entries MUST use the `(hanzi, tl)` pair (project-wide: `lexicon`, `ranking`, dictionary `merge`/`merge_csv`/`cleanup`, accent generation). POJ/TPS are alternate renderings of the same TL and add no new identity. Full statement + rationale: project `CLAUDE.md` Core Principle #7.

## Required reading (in order)

1. **`knowledge/taigi-phonetics-reference.md`** — full file. Especially:
   - §2 Initials table
   - §3 Finals table (esp. §3.2.6 Special/Dialectal Finals: `irinn`, `irk`, `irp`, `irt`, `irm`, `irn`, `irng`, `er`, `erh`, `erm`, `ee`, `ere`, etc.)
   - §4 Cross-Reference
   - §5 Tones table — TPS uses diacritic characters (full list under "Key TPS facts" below). Do NOT assume ASCII digit-tone encoding.
2. **`engine/phonetics/src/<system>.rs`** — source-of-truth lookup table for each system. e.g. `tps.rs` defines `ZHUYIN_INITIALS / ZHUYIN_VOWELS / ZHUYIN_TONES / ZHUYIN_TONES_ENCODE_SAFE`.
3. **`knowledge/tps-auto-correct-rules.md`** — if touching TPS (palatalization, nasal, coda-position rules).
4. **`taigi-converter/src/`** — canonical TL ↔ POJ ↔ TPS converter (git submodule). Plus `dictionary/common/taigi_bridge.py` for the Node-IPC bridge.
5. **THEN, finally**, `AskUserQuestion` — only for user-decision forks (product strategy / naming preference / scope). Do NOT ask questions whose answer is in the reference.

## Key TPS facts (often-misremembered)

- **TPS is NOT Mandarin bopomofo.** It borrows Unicode Bopomofo (`U+3100–U+312F`) + Bopomofo Extended (`U+31A0–U+31BF`) but is a distinct writing system.
- **TPS has Taiwanese-specific initials** not present in Mandarin bopomofo: `ㆠ` (b), `ㆡ` (j), `ㆢ` (ji palatalized), `ㆣ` (g), etc.
- **TPS tones are diacritic characters**, NOT ASCII digits:
  - tone 2: `U+02CB ˋ`
  - tone 5: `U+02CA ˊ`
  - tone 3: `U+02EA ˪`
  - tone 7: `U+02EB ˫`
  - tone 6: `U+02C7 ˇ`
  - tone 8: `U+0307` or `U+02D9 ˙`
  - tone 9: `U+02C6 ˆ`
- **Entering and stopped finals** (tones 4 and 8) use dedicated symbols: `ㆴ ㆵ ㆻ ㆷ`.
- Multiple position-dependent forms (initial vs coda) — `ㄇ→ㆬ`, `ㄋ→ㄣ`, `ㄅ→ㆴ` etc. See `knowledge/tps-auto-correct-rules.md` Rule 2.
- **TPS = hanji-first input mode**, equivalent to `is_translate_swapped`-class IMEs. The engine treats `input_mode == "tps"` as effectively swapped (see `engine/composing/src/api.rs` around the `effective_swapped` derivation). Do NOT propose "TPS displays TL roman".

## No "dead code" inference from absence

When auditing phonetic tables (`TL_INITIALS`, `TL_FINALS`, `TONE_NUM_TO_COMBINING`, POJ suffix sets, TPS maps, etc.), do NOT use "no test covers it" + "no dictionary word uses it" as evidence of dead code. The dictionary is a current snapshot; the table is a linguistic contract.

- **Before proposing to remove anything from a phonetic table**: read `knowledge/taigi-phonetics-reference.md` end-to-end first.
- **If a final/initial/tone follows the established pattern but isn't in the doc**: ask the user. Don't infer.
- **"No test covers" alone is not evidence.** Tests cover sample cases, not the full phonetic surface.
- **"No dictionary word uses it" alone is not evidence.** Dictionaries grow; the parser must already accept the syllable when a future word lands.
- For drift triage: default to "(a) canonical missing" before reversing iOS/Android.

**Why**: 2026-04-26. Auditing taigi-converter `iri/erk/eeh` drift, I scanned the 1151 dictionary syllables, found no `*-iri / *-erk / *-eeh`, and proposed removing them everywhere. User corrected: "iri/erk/eeh 這個有意義，是特殊字尾". They're legitimate special/dialectal finals per §3.2.6.

**Why (TPS)**: 2026-05-20 三索引 round. I proposed a TPS schema with `tps_num = digit-tone` and listed `Zhuyin (bopomofo) tps_num = ㄏㄛ2ㄙㄝ3` as an `AskUserQuestion` option. USER rejected twice: (1) "TPS 有自己的聲調表示方法,不是用數字輸入"; (2) "TPS 不是 bopomofo,TPS 和 bopomofo 是不同的系統". I hadn't read §5 Tones or `tps.rs::ZHUYIN_TONES` — the answer was already in the reference.

## Anti-patterns

- Inferring schema from training memory → proposing `AskUserQuestion` options that contradict `knowledge/`.
- Asking the user a fact that the reference already answers.
- Treating dictionary/test absence as licence to delete table rows.
- Conflating TPS with Mandarin bopomofo or with TL/POJ.
