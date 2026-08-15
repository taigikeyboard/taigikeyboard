# macOS IME — Kickoff Handoff

Paste the **Kickoff Prompt** below as the first message of a fresh Claude Code session
to start the macOS desktop IME. This is a **Phase 0 (research + plan)** kickoff — the
session must NOT write app code until you approve the plan.

Recommended model / effort is at the bottom. **Phase 0 is intended for Claude Fable 5**
— the prompt is written goal-and-constraints first (not step-by-step), which is how
Fable does its best work; it also runs fine on Opus 4.8.

---

## Kickoff Prompt

```
Context: Taigi Keyboard = cross-platform Taiwanese IME. iOS (Swift+KeyboardKit) +
Android (Kotlin+FlorisBoard) share a Rust engine (`engine/`) over FFI (swift-bridge
xcframework / JNI). I'm adding a macOS desktop IME as a NEW platform. This is a
greenfield, multi-PR effort. Treat this turn as Phase 0: research and plan only — do
NOT write app code, do NOT branch, until I approve the plan.

GOAL
- New `macos/` folder. A native macOS input method on Apple InputMethodKit
  (IMKServer / IMKInputController) — NOT KeyboardKit (that's iOS-only).
- Reuse the shared Rust `engine/` (phonetics / composing / lexicon / ranking /
  nextword). Do NOT reimplement any Taigi logic in Swift.
- Desktop = TL + POJ only. NO TPS (方音符號).
- Keyboard shortcuts follow mainstream macOS convention (VSCode / Sublime Text).
- Settings reachable BOTH via a keyboard shortcut AND by clicking the IME menubar icon.
- Custom-dictionary support (reuse the engine custom-dict contract + the SQLite
  pattern iOS/Android already use).
- Modern SwiftUI UI/UX, macOS 14+ HIG.

HARD CONSTRAINTS — do not violate
1. Do NOT modify `ios/` or `android/`. Another session is actively editing them.
   Especially DO NOT touch anything theme-related, or shared `i18n/` / `content/`.
2. `engine/` is SHARED. Prefer reusing the existing xcframework as-is. If macOS needs
   a new FFI export, make it PURELY ADDITIVE and flag it as a shared-surface change
   for me to coordinate — never refactor existing engine FFI.
3. No release / tag / store actions — I release macOS only when I say so.

WHY THIS MATTERS
The shared Rust engine is the whole point of this codebase — the value of the macOS
port is proving the engine reuses cleanly on a third platform with a thin native
shell. Getting the engine-reuse boundary and the IMKit architecture right up front is
what this phase exists to do; a wrong boundary is expensive to unwind later.

REFERENCES — read and cite `file:line`
- `references/khiin-rs/swift/osx` — a Taiwanese macOS IME (IMKit + Rust engine).
  Closest prior art. Study its IMKit wiring + how it consumes the Rust engine.
- `references/MacishType` — macOS IME reference impl.
- Repo: `engine/` (FFI boundary, `engine/swift-ffi`, xcframework build in the Makefile),
  `docs/architecture/`, `docs/engine/`, `.claude/rules/`.

DELIVERABLE FOR THIS TURN
A plan I can approve — not code. It must include:
- A grounded (file:line) design: engine-reuse boundary, macOS app architecture
  (IMKInputController → composing state → engine → candidate window), settings
  surface, custom-dict storage, macOS shortcut scheme.
- Whether a macOS target can reuse the existing xcframework or needs a new (additive)
  FFI surface, decided from the actual `engine/` FFI iOS consumes.
- The TL/POJ-only composing path (which engine calls, TPS excluded).
- A Codex pre-impl design review of the plan (my Codex sandwich).
- A git-tracked roadmap doc + a project-memory topic file, phased multi-PR (Phase 0 =
  scaffold `macos/` + engine-reuse spike; then composing → candidate UI →
  settings/menubar → custom-dict → polish).
Decide your own research approach; use a parallel agent team where the work is
genuinely independent. Present the plan via ExitPlanMode for approval.

Batch all clarifying questions into your first turn. Report progress grounded in what
you actually read (cite file:line); don't claim what you haven't verified.
```

---

## Recommended model / effort

| Stage | Model | Effort | Why |
|---|---|---|---|
| **Phase 0 (this prompt): research + architecture + roadmap** | **Claude Fable 5** (`claude-fable-5`) | **high** (bump a single design-review step to `xhigh` only if needed) | Built for long-horizon, high-ambiguity, agent-team work — the engine-boundary + IMKit architecture decision is exactly this. Bounded one-time cost; getting direction right is worth the premium. |
| Later impl phases (scaffold / wiring / SwiftUI) | Opus 4.8 main + Sonnet 4.6 subagents for boilerplate | medium–high | Fable is overkill for mechanical wiring; keep IMKit-lifecycle + engine-FFI work on Opus. |

Notes for running Fable:
- Single requests on hard tasks can run several minutes — that's normal, not a hang.
- Fable prefers goal+constraints over step-by-step (why this prompt is written that
  way); don't re-add prescriptive step lists.
- Set effort with a sweep in mind — default `high`, reserve `max` for genuinely hard
  verification, not routine work.

## Cross-session watch-outs

1. `engine/` is the only shared-surface conflict risk. macOS FFI needs must be
   additive-only + flagged for coordination; never refactor existing engine FFI.
2. New session auto-loads the trimmed `MEMORY.md`. Give macOS work its own memory topic
   file per planning.md — do not fold it into existing i18n entries.
3. Do NOT touch theme / `i18n/` / `content/` — owned by the concurrent ios/android session.
