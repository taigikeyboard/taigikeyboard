# Codex Strategic Review — Shared-Core Extraction Roadmap

**Date**: 2026-04-19  
**Subject**: 5-phase roadmap + Phase I plan + `/shared-core-confidence` skill design  
**Verdict**: proceed with modifications, 87% confidence  
**Agreement**: all Critical + Important + Minor findings accepted (some framing adjustments).  
**Apply status**: roadmap revised to 6 phases (Phase 0 inserted, Phase IV split to IV-A/IV-B); Phase I plan revised to add G0/G10 and promote G9 to hard gate; skill dimensions reweighted with new D9.

This doc preserves Codex's full findings for reference before each phase begins.

---

## Critical (must resolve before Phase I starts)

### C1 · Missing non-functional gates
The roadmap is missing explicit non-functional gates for keystroke latency and memory, which is a major flaw for an IME with Unicode-heavy logic and an iOS keyboard extension capped at 64MB. The current gates are mostly structural purity checks, parity checks, and portability heuristics, but none directly prove that segmentation DP, trie lookup, normalization, and next-word logic stay within an acceptable latency budget per keystroke. A refactor that improves architecture while regressing hot-path latency or memory residency would still pass your current plan and fail the product.

**Applied**: G0 captures baselines; Phase I gating signals 6 & 7 enforce no-regression.

### C2 · iOS exemplar first bakes Swift assumptions into the shared contract
The "iOS exemplar first" strategy is only safe if you define a neutral behavioral contract before Phase I, otherwise you risk encoding Swift/KeyboardKit assumptions into the future shared core. Your candidate shared files are Foundation-only and the audit is clean, but that does not mean the surrounding boundaries are platform-neutral once DI, lifecycle, timing, and state snapshots are reshaped around iOS first. The best argument against the current order is that Android will end up copying an iOS-shaped abstraction rather than a domain-shaped one, which raises later extraction cost even if Phase II appears successful.

**Applied**: new Phase 0 requires `docs/architecture/behavioral-invariants.md` before Phase I begins.

### C3 · 95% confidence can certify cleanliness without proving FFI viability
The 95% confidence threshold is not strong enough by itself because it can certify architectural cleanliness without proving FFI viability under real platform constraints. D3, D4, and D6 help, but nothing in the described gate forces an actual end-to-end probe of Rust inside the iOS extension and Android app with realistic dictionaries, Unicode normalization, and user-frequency storage. Risk: you can hit 95% on paper and still fail on build complexity, symbol size, allocator behavior, or cross-language string/SQLite edge cases once Phase IV starts.

**Applied**: D9 (FFI POC gate, 5%) added; Phase III requires BOTH ≥95% AND D9 pass. Phase IV split into IV-A (Phonetics-only Rust proof) and IV-B (full extraction).

### C4 · Rust operational risk underestimated
Rust is a defensible target, but the roadmap underestimates Rust-specific operational risk for this domain and deployment model. The hardest problems are not raw algorithm portability but Unicode equivalence behavior, binary dictionary portability, and embedding a native core inside constrained mobile surfaces with stable interfaces over app updates. Hypothesis: the biggest Rust risks are not correctness of the pure algorithms, but integration drag from UniFFI/JNI/Swift bindings, memory overhead around strings and buffers, and divergence between platform SQLite usage and whatever boundary you choose for persistence.

**Applied**: G10 (data artifact portability audit) tracks Unicode/MARISA/SQLite boundary decisions starting in Phase I; final integration risks resolved in Phase IV-A.

---

## Important (revisit within Phase I)

### I1 · G4 scheduled last misses boundary-design leverage
G4 scheduled last is risky because `ComposingManager` is the hot path and likely the highest-value place to discover hidden UI/engine coupling early. A better framing: surface G4's boundary design early and de-risk it before easier DI cleanups shape the architecture around the wrong seam — not necessarily implement G4 first in full.

**Applied**: G4 and G5 split into `-design` (front-loaded sketch before G1/G2/G3/G6) and `-impl` (late execution).

### I2 · G4/G5 are behavioral-boundary design, not class splitting
G5 and G4 should be treated as behavioral-boundary design work, not merely class splitting, because timing and actor semantics can distort IME behavior after extraction. `@MainActor`, `Timer`, `ObservableObject`, and `@Published` mean some current behavior may be emergent from platform scheduling rather than domain logic. Risk: once decay timing or composition updates are detached, the system may feel different to users in ways the current scoring model would not catch.

**Applied**: G4/G5 design deliverables are docs (`composing-state-boundary.md`, contract inside plan) rather than code sketches.

### I3 · D6 under-weighted relative to Rust objective
D6 is underweighted relative to the stated objective of Rust extraction. If Rust is the target, portability blockers deserve more than 15% — but D3 should still dominate slightly, because a portable but behaviorally divergent core is useless for an IME.

**Applied**: D6 raised 15 → 20%; D3 kept at 20%; D1 and D5 reduced to free the budget.

### I4 · Dictionary / MARISA / SQLite portability missing
The roadmap is missing a concrete plan for dictionary and lexicon artifact portability across Swift, Kotlin, and Rust. MARISA tries and SQLite-backed user frequency have no explicit phase for binary format compatibility, load-time cost, update strategy, or fallback behavior. This kind of problem appears "below the architecture layer" until it blocks extraction.

**Applied**: G10 added (Phase I deliverable).

### I5 · Android state assessment missing before Phase II
"Android can copy the iOS exemplar shape" assumes Android's current lifecycle and data-flow state is close enough. D2 file-level parity does not prove state ownership or threading parity. Without a concrete Android state assessment, Phase II may be more translation than alignment.

**Applied**: Phase II gating entry in roadmap now requires an Android state audit doc as the pre-phase deliverable (mirror of this plan).

### I6 · One-shot Phase IV too aggressive
A one-shot extraction in Phase IV is too aggressive. Phonetics and conversion logic are better candidates for an early FFI proof than composition state or next-word ranking. Proving one low-coupling slice first gives more signal than a composite confidence score alone.

**Applied**: Phase IV split to IV-A (Phonetics slice) and IV-B (remaining candidates).

### I7 · G9 "recurring" is too soft
G9 being "recurring" is too soft for a refactor whose stated endgame is shared-core extraction. Cross-platform golden tests and engine baselines are not cleanup work — they are the only reliable defense against invisible behavior drift in NFD handling, TL/POJ/TPS conversion, segmentation, and next-word logic. If tests are not front-loaded, the exemplar can become structurally elegant while silently redefining behavior that Android then copies.

**Applied**: G9 promoted from "recurring" to Phase I hard gate; must close before Phase II; references Phase 0 invariants by name.

---

## Minor (track)

- **M1**: grep-based purity gates are gameable once teams learn the allowed shapes; keep as hygiene, not extraction-readiness evidence. *Applied*: D1 weight reduced 15 → 10%.
- **M2**: `// INVARIANT:` tags are documentation, not guarantees — D5 weight too high unless linked to automated tests. *Applied*: D5 reduced 15 → 10% and semantics changed to "tag references a named test".
- **M3**: debug/log parity and symbolication deserve explicit ownership once a Rust core exists. *Tracked*: roadmap "Open concerns tracked" section, revisit in Phase IV-A design.
- **M4**: ABI stability over app updates is a release-management policy concern more than an architectural blocker now. *Tracked*: Phase IV-B release policy.
- **M5**: staying duplicated is not obviously irrational if the shared slice remains ~2400 LOC; break-even depends more on hot-path ratio than LOC. *Tracked*: Phase III gate explicitly requires break-even analysis before green-light.

---

## Done well (what is sound in the plan)

1. Refactoring before extraction is the right broad sequence. A clean Foundation-only candidate set with zero compile-time soft dependencies means you are not starting from a tangle where extraction would just relocate existing mess into Rust.
2. iOS-first is defensible because the iOS keyboard extension is the harsher runtime environment. Architecture that survives the 64MB extension constraint is less likely to fail on Android for resource reasons.
3. The Phase I task list targets actual coupling points rather than cosmetic reorganization. `BackupService` DI, `SharedSettings` reads, `NextWordController` timing, and `ComposingManager` hot-path state are exactly where fake modularity hides.
4. The skill dimensions are aimed at the right failure modes. Behavior alignment, data model parity, portability blacklists, and external dependency scanning are materially better signals than raw LOC moved or module counts.
5. Planning Android to mirror iOS model shapes after the exemplar can reduce cross-platform ambiguity in Unicode handling and candidate generation — provided "same shape" does not become "same platform-biased abstractions".

---

## Alternative strategies Codex surfaced (not adopted as-is, but useful lenses)

1. **Neutral architecture spec first, iOS as first implementer**: adopted as Phase 0 instead of a full spec — a lighter invariants doc suffices for an IME whose behavior is already implemented twice.
2. **Staged extraction proof before committing to full Rust**: adopted as Phase IV-A Phonetics FFI proof.
3. **Benchmark Kotlin Multiplatform as an alternative even if Rust is still chosen**: not adopted yet; revisit before Phase IV-A if FFI integration drag appears severe in the POC.
4. **Parallelize only the contract/spec work, not the refactor work**: effectively adopted via Phase 0 sequencing.

---

## Final recommendation (Codex)

Phase I should not start until you add:
- (a) neutral behavioral contracts before the iOS refactor shapes the shared boundary,
- (b) hard latency and memory gates alongside structural purity gates,
- (c) a front-loaded (non-recurring) engine test baseline,
- (d) a staged extraction proof replacing the single 95% confidence green-light.

The broad sequencing is defensible; the gates and pre-conditions are the gap.

**Confidence: 87%.**

All four pre-conditions applied to the revised plan.
