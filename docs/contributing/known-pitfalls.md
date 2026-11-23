# Known Pitfalls

Technical rules distilled from past incidents. The dated narratives that produced them live in `docs/architecture/incident-log.md`.

## Tests and diagnosis

- **Trace before assert**: compute an expected value from the real tables (`engine/phonetics/src/tables.rs`), never from the code's actual output. `tables::tl_tone_mark` returns "" for tones 1 / 4, so `tl::to_tl` leaves those syllables unmarked.
- **Fixture rule**: a syllabifier / continuous-fetch / golden fixture exercising syllable `X` must also include every production syllable that is a strict prefix of `X` and assert its absence (or presence). Confirm against production artifacts via `engine/composing/tests/candidate_dump.rs` first.
- **Fix-location rule**: display-only candidate strips belong in the span-local key builder (`composing::shadow::keys::left_anchored_keys_and_restrictions`), never in the `syllabifier` span-ending primitives (`syllabifier::tl::valid_span_endings`, `syllabifier::valid_span_endings_lowered_with_barriers`) — touching the primitive reintroduces the `span_min_syllable_count("tania")` regression.
- **Verify pipeline claims**: grep production data and run a real query before trusting a document's "current behaviour". `dictionary/common/notone.py::remove_tone()` already strips digits and hyphens; `dictionary/build/merge_csv.py` `groupby((hanzi, _tl_key))` already enforces `(hanzi, tl)` uniqueness.
- **Unmerged or unbuilt prerequisites**: a path whose prerequisite PR has not merged, or whose binary was not rebuilt, never ran — static analysis is not verification.
- **Three failed fixes for one bug**: stop adding mitigations and compare against a working reference (`references/aiongtaigi-sushi`, `references/florisboard`).
- **Spike a platform capability before planning on it**: a third-party iOS keyboard extension never receives hardware-keyboard `UIPress` events (`pressesBegan` on `UIInputViewController` is dead on device; iPadOS delivers them to the host app only), so iPad external-keyboard composing is impossible in the extension. Any plan resting on an unverified OS capability starts with a ≤20-line on-device spike. The same goes for subclassing a system control and overriding its input handling: before relying on which view receives a click, a key or focus, run a ≤20-line probe on the current OS that prints the control's `subviews` and the window's `hitTest` result. Current `NSSearchField` hosts a SwiftUI view and an `_NSSearchFieldClearButton` that take clicks before the field's own `mouseDown`.
- **UI event tests go through the OS dispatch entry point**: deliver a click through `window.sendEvent` (mouse-down then mouse-up; assert the window's `hitTest` returns the view under test), focus through `makeFirstResponder`, keys through `NSApp.sendEvent` — never call the overridden `mouseDown` / `keyDown` / `becomeFirstResponder` / `beginRecording` directly. A direct call skips the hit-test, field-editor and responder-chain steps where AppKit bugs live — `sendEvent` hands first responder to a view that accepts it before `mouseDown` runs — so the test stays green while the device fails. A test process is never the active app, so a custom `NSView` that does not answer `acceptsFirstMouse` true (every system control does) never sees a sent click. Direct calls stay fine when the test is about the method's own contract (a no-window guard, idempotent teardown). Template: `click(_:atX:)` in `macos/Tests/TaigiInputMethodCoreTests/ShortcutKeyRecorderTests.swift`.

## Review and device gates

- A diff carrying numeric / geometry fidelity risk (wide caller surface, unit conversion) gets a dedicated line-by-line review — a double `toInt()` truncation once passed a design-level review.
- Qualitative perf gate: **S1 POJ diacritics**, **S2 TPS composition**, **S3 Hanji candidate scroll**, plus the iOS keyboard-extension 64 MB hard cap, leak-free, no keyboard dismiss. Root-cause receipts live in `docs/architecture/behavioral-invariants.md`.

## Design

- Read the module's entry point (`api.rs` / `requests.rs`) before proposing integration; `engine/nextword` only filters / scores the rows it is fed and fetches nothing — the platform's `PredictNext` is expanded with the lexicon lookup and user rows in `engine/dispatch/src/predict.rs` before it reaches nextword.
- Cite best practices from `docs/references/mainstream-ime-comparison.md` first. Proven cites: `references/khiin-rs/khiin/src/buffer/buffer_mgr.rs` (commit-and-resegment), `references/librime/src/rime/...` (segment status state machine), `references/aiongtaigi-sushi` / `references/florisboard` (IME window / inset).

## Phonetics

- Never remove or infer a TL / POJ / TPS rule from dictionary or test absence (`docs/contributing/phonetics.md`). TPS has its own tone marks (`engine/phonetics/src/tps.rs::ZHUYIN_TONES`) — not digit tones, not bopomofo.

## Privacy

- No personal identifier in any tracked file, commit message, PR or issue — write "a former work address", "the maintainer's author email". The maintainer's private denylist gates tracked files, commit messages, and the title and inline body of a pull request opened through Claude Code; check issues, and any other pull request text, by hand. Rules and gates: `CONTRIBUTING.md` § Secrets.
