# Real-text TPS typing simulation — Windows + Android (2026-10-10)

Test-only run. No branch, no fix. Findings are listed for the maintainer to decide which become rounds; each is tagged **observed failure** or **expected behaviour**.

## Setup

| | Windows (dev box) | Android (Samsung SM-A5560, wifi adb) |
|---|---|---|
| Build | `56fb00ef` (#477), `install-dev.ps1 install` | `56fb00ef` after `make build`, `:app:assembleDebug`, same-cert `adb install -r` |
| Binary gate | No log-out (unsaved host document open). Instead: the driver's own host started after install; in-process `TaigiKeyboard.dll` path + sha256 = fresh build. `dumpbin` not run. | device `dictionary.bin` md5 = repo artifact |
| TPS behaviour | arm B (Hanji conversion in preedit, shipped default) | mobile strip |
| Host | driver-owned Tk window (session 1, foreground guard every key) | Samsung Notes, new notes |
| Learning state | box history + 17,674 custom rows (see W6) | clean (0 rows) at start |
| Segments | 100 per row (829 Hanji) | F 53 / G 56 (subset; tapping is slow) |

Text: `corpus/taigi-typing` — 3 `mapped` articles (three authors) + 30 random example sentences, split at punctuation into 100 segments. TPS keys from `taigi-converter` (`tai tl tps`). Expected output = corpus Hanji. Rows: **F** = TPS toneless, **G** = TPS with a tone mark on every syllable. Random 80–400 ms inter-key gaps.

Pick strategy (fixed): slot 0 right → take; else first 9 rows of the window; else shorter span / more pages; else (F) retype toned once.

## Summary

| | Win F | Win G | Android F | Android G ¹ |
|---|---|---|---|---|
| first candidate exact | **11 %** | 72 % | 49 % | 64 % |
| char accuracy, first conversion | 70.6 % | 95.1 % | — | — |
| char accuracy, committed | 96.3 % | 96.4 % | 98.3 % | 98.9 % |
| keys (taps) per Hanji | 7.11 | 3.77 | 3.41 | 3.59 |
| crashes / dismiss / stuck | 0 | 0 | 0 | 0 |
| key → preedit latency | median 17 ms, max 78 ms | median 14, max 33 ms | no visible lag | no visible lag |

¹ Android G typed the same sentences after F, so F's picks inflate G. F is the clean number.

Android IME PSS: 173 MB before → 238 MB after F → 209 MB after G (no monotonic growth).

Clean-engine control (`engine/composing/tests/candidate_dump.rs`, `DUMP_MODE=tps`, production artifacts, no user data) over the same 200 inputs: slot 0 exact on G 64/100, F 38/100 (the dump has no Space tone-1/4 pin on the last syllable, see W2).

## S120 (§18 guard (e), #477)

| Check | Windows | Android |
|---|---|---|
| `ㄌㄧㄏㄛ` → 你 among one-syllable rows | PASS — 你 #8 (page 1) after 你好 字號 理會 裂 哩 … | PASS — 你 strip index 4 |
| 汝 for `ㄌㄧㄏㄛ` | not listed | not listed (7 pages) |
| `ㄌㄧㄏㆦ` → 你 beside 裂 | PASS — 裂 #1, 你 #5 | PASS — 你 #2, 裂 #9 |
| `ㄍㄚㄉㄚ` → 家 / 加 beside 結 | PASS — 結 #1, 家 #16 / 加 #17 (page 2) | PASS — 家 #4 / 加 #5, 結 #9 |
| `ㄍㄚㆵ` alone → no one-syllable 家 / 加 | PASS (two-syllable 家己 / 家庭 … are prefix completions) | PASS (same) |
| `ㄌㄧˋㄏㄛˋ` → 你好 | PASS | — |
| Pick 你, Space → 你熇 | as written | — |

汝 is absent because `汝,lí` is `is_variant=True` (`dictionary/output/dictionary.csv:38`) and the Variant source is off by default; `candidate_dump` with every source lists 汝好 / 汝. **Expected behaviour**; the S120 text "你 / 汝" holds only with Variant on (checklist wording).

## Findings (severity order)

### 1. TPS: a typed tone mark is dropped when the span's last syllable is unmarked — HIGH, observed failure (Android F1)

| Keys | Slot 0 | Expected |
|---|---|---|
| `ㄉㄞ˫ㄨㄢ` | 台灣 tâi-uân | 大員 tāi-uân (typed tone 7) |
| `ㄗㄞˊㄙㆥ` | 在生 tsāi-senn | tsâi… (typed tone 5) |
| control `ㄉㄞ˫ㄨㄢˊ` | 大員 | ✓ |
| control TL `tai7uan`, POJ `tai7oan` | 大員 | ✓ |

Device: G row `…ㄌㄤˊㄗㄞˊㄙㆥ…` → 人在生. Reproduced with `candidate_dump`.
Suspicion: `engine/composing/src/shadow.rs:812` `span_tone_pin` applies `TonePin::TypedTones` (§17 case 3) to `InputMode::Tl | InputMode::Poj` only; TPS gets `TpsSpaceEnd` (`:822`) or `None`, so a partial-tone TPS span is looked up toneless. Unconfirmed — needs the root-cause gate.

### 2. TPS: tone mark / Space does not stop the next initial being read as the previous syllable's coda — HIGH, observed failure (Windows W1)

| Keys | Slot 0 | Expected |
|---|---|---|
| `ㄌㄧˋㄫㆤ˫` | 靈會 lîng ē | 你硬 lí ngē |
| `ㄌㄧ␣ㄫㆤ˫` | 靈會 | 你硬 |
| `ㄍㄚˋㄍㄚ` | 角仔 kak-á | ká ka… |
| `ㄏㄛˋㄇㄚ` | 號碼 hō-má (typed tone 2 → 7) | hó ma… |
| controls `ㄫㆤ˫` → 硬; `ㄌㄧˋ` → 你; `ㄍㄚˋㄉㄚ` → ká ta | ✓ | |

Box step trace: `ㄌㄧˋ` → 你, `+ㄫ` → `你ㄫ`, `+ㆤ˫` re-walks to 靈會. Reproduced with `candidate_dump`.
Suspicion: the §35 dual-form family (`engine/phonetics/src/tps_ambiguity.rs:129-130` ㄫ+ㄧ→ㄥ; `engine/phonetics/src/tps_adjust.rs:199` `dual_final_form`) is applied across a tone-mark / Space barrier in the shadow lattice (`engine/composing/src/shadow.rs` barrier handling ~`:223-267`), contradicting `docs/architecture/desktop-tps-roadmap.md:102` ("only a tone mark or Space pins the boundary"). May share a cause with finding 1 (both: typed tone not honoured in TPS spans). Unconfirmed.

### 3. Desktop toneless TPS has no neutral convert key — HIGH UX cost, expected behaviour (Windows W2)

Space closes the reading as tone 1/4 (`engine/composing/src/conversion.rs:7`, S120(a) "Space → 你熇"). Row F: last character wrong in 72/72 segments whose last syllable is not tone 1/4, vs 1/28 when it is (G control 4/72). `ㄏㄛ␣` → 熇, list only tone-1 rows (no 好). Without the pin the clean engine gets 38/100 F exact vs 11/100 on the box. The toneless user must ↓ and pick (`ㄌㄧㄏㄛ ↓ 1` → 你好). Android toneless reached 49 % first-hit because the strip converts without that pin. **Decision for the maintainer**: whether desktop toneless TPS gets a neutral convert path.

### 4. Enter on an unconverted desktop composition writes the display-only ㆷ — MEDIUM, observed failure (Windows W3)

`ㄌㄧㄏㄛ` Enter → document gets `ㄌㄧㆷㄛ` (typed ㄏ replaced). The fold is meant as preedit display (`tps_adjust.rs:129` via `transition.rs:338`), but commit-as-shown writes it. Control `ㄌㄧˋㄏㄛˋ` Enter → 你好. Relates to the open "desktop fold skip" idea in the S120 round.

### 5. Wrong homophone in slot 0 — MEDIUM, observed failure (ranking; both platforms)

| Keys | Actual | Expected | Note |
|---|---|---|---|
| `ㄌㄤㄗㆤㆤ` | 人坐的 | 人濟的 | 濟 freq 6126 > 坐 3988 (`dictionary.csv:335/533`) |
| `ㄌㄠˊㄏㆦ˫ㄌㄧˋ` | 流予你 | 留予你 | toneless gives 老狐狸 |
| `ㄒㄧㆩˋㄍㄠ˪` | 啥到 | 啥夠 | 夠 not offered at usable rank |
| `ㄨㄢㆠㄛㄒㄧㄨ` | 完無受 | 無冤無仇 | idiom in dictionary (freq 12) |

Windows G also: 啊/矣, 佮/甲, 个/的, 誠/成, 到/夠. Most reproduce on the clean engine. Suspicion: walker edge cost (E1 corpus frequency) vs dictionary frequency in `continuous.rs`. Needs a trace before any by-design call.

### 6. 真/眞 and 類/纇 look identical in the Android default candidate font — LOW, observed (display)

Engine rank is correct (真 first); the user can still pick the wrong twin.

### Not findings
- Windows: non-BMP 𪜶 lost at commit in the Tk test window only; a native Win32 EDIT commits `𪜶兜` correctly (host limit).
- Android: one swallowed Enter not reproducible; one burst of ignored taps then stray toolbar taps (layout/theme changed) not reproducible manually — driver-side, guarded afterwards.
- Mobile preedit shows folded `ㆷ` (expected, S120(a)).

## Method limits

- Windows candidate reading = template match (OCR misses single characters); picks are a lower bound — at least 16 of 118 "not in first 9" windows probably held the target.
- Windows first F pass used a wrong row pitch: 24 wrong picks were learned into the box's learning records (e.g. *ni* → 拈 / 耳 / 爾, *kui* → 鬼 / 歸) and are visible in G (one 染 → 耳). Those runs are excluded; the box's learning records need a manual cleanup (a pre-run AppData copy exists on the box).
- Android G inflated by F's learning (G typed the same sentences after F).

## Device state after the run

- Windows: `settings.json` restored, byte-identical to the backup. TIP CLSID still points at the dev build (was the installed 3.7.0); old dev dictionaries kept as `release\Dictionaries.old-20261010`; learning records contain this run's picks.
- Android: prefs file byte-identical to the backup, decoded diff empty; learning records deleted (0 rows, as at start); 10 test notes in the Samsung Notes trash (not emptied); default IME restored to TaigiKeyboard (the state found at start).

Raw data and screenshots stayed in the session scratchpad (corpus licensing: not committed).
