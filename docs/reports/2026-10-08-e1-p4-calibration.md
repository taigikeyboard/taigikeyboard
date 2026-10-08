# E1 P4 — Walker Calibration

> **Type**: Report (dated snapshot — frozen)
> **Keywords**: `walker`, `walker_cost`, `alpha`, `calibration`, `walker_gold`, `final`
> **Date**: 2026-10-08 · branch `docs/e1-p4-calibration-report` (E1 P4) off `main` `05fec018`
> **Plan**: [`architecture/unified-word-frequency-roadmap.md`](../architecture/unified-word-frequency-roadmap.md) §6 P4 · **Previous**: [P3 report](2026-10-08-e1-p3-walker-cost.md)

P4 calibrates the walker model shipped in P3: the smoothing α, `CUSTOM_EDGE_COST`, the single-syllable user-delta scale and the khiin length exponents. Result: every constant stays as P3 shipped it. No artifact, constant or test changes; this report and two comments are the whole phase.

## Summary

- **α stays 10.** An engine grid over {0.5, 2, 10, 50} on all eight input variants puts α 10 first on `dev` (79.9 % exact / 87.1 % segmented over 2,112 inputs) and first or tied on `calib` (75.7 / 81.3 over 544).
- **α 50 was briefly chosen and reverted.** The offline simulator, which prices TL full tone only, favoured 50 (`calib` segmented 96 → 97 %, `dev` 93 → 94 %). The engine harness showed that 50 gains about one full-tone item and loses on every toneless variant (`dev` TL toneless 70.5 / 76.1 → 69.7 / 75.0, `calib` 61.8 / 64.7 → 60.3 / 61.8). The maintainer confirmed α 10 on 2026-10-08.
- **`CUSTOM_EDGE_COST` (7,878), the user-delta scale (0.0) and the length exponents (0.2) stay.** The gold set has no custom, learned or selected entries, so `calib` cannot move the first two. No length-exponent change was tried, because the plan changes them only on a `calib` win.
- **`final` read once** (α 10, 82 items, engine): TL full tone 85.4 % exact / 91.5 % segmented, TPS full 79.3 / 84.1, toneless 47.6 / 50.0. The simulator on the same items: pre-E1 85 / 88 % → E1 85 / 90 % (TL full tone).

## 1. α grid (engine, `walker_gold.rs`, default sources)

Exact % / segmented %. Each column is a separate `make dict` at that α, then `cargo test -p composing --test prod walker_gold -- --include-ignored`. The α 10 column re-measures P3 on the same harness, and it matches the P3 report.

| Split | Variant | n | α 0.5 | α 2 | **α 10** | α 50 |
|---|---|---|---|---|---|---|
| dev | tl_full | 264 | 85.6 / 96.2 | 85.6 / 96.2 | 86.4 / 97.0 | 86.7 / 97.0 |
| dev | tl_partial | 264 | 83.3 / 93.6 | 83.7 / 93.9 | 85.6 / 96.2 | 86.0 / 96.2 |
| dev | tl_toneless | 264 | 66.7 / 72.7 | 68.2 / 73.9 | **70.5 / 76.1** | 69.7 / 75.0 |
| dev | poj_toneless | 264 | 66.3 / 72.3 | 67.8 / 73.5 | **69.7 / 75.4** | 68.9 / 74.2 |
| dev | tps_full | 264 | 79.2 / 87.1 | 80.3 / 88.3 | **82.2 / 90.5** | 82.2 / 90.2 |
| dev | tps_toneless | 264 | 67.0 / 73.1 | 69.3 / 75.0 | **71.6 / 77.3** | 70.8 / 76.1 |
| dev | all | 2,112 | 77.5 / 84.5 | 78.3 / 85.3 | **79.9 / 87.1** | 79.8 / 86.5 |
| calib | tl_full | 68 | 85.3 / 95.6 | 85.3 / 95.6 | 85.3 / 95.6 | 85.3 / 97.1 |
| calib | tl_toneless | 68 | 61.8 / 64.7 | 61.8 / 64.7 | **61.8 / 64.7** | 60.3 / 61.8 |
| calib | tps_full | 68 | 77.9 / 85.3 | 79.4 / 86.8 | 77.9 / 85.3 | 77.9 / 85.3 |
| calib | tps_toneless | 68 | 60.3 / 63.2 | 61.8 / 64.7 | **63.2 / 66.2** | 61.8 / 63.2 |
| calib | all | 544 | 75.4 / 80.9 | 75.7 / 81.3 | **75.7 / 81.3** | 75.2 / 80.7 |

`poj_full` equals `tl_full` at every α. `tl_hyphen` (exact only) runs 0.3–0.4 points above `tl_full` on `dev` at every α. Both are left out of the table.

Exposure screen (`tools.walker_gold exposure`, every fully toned 2–3 syllable key): the keys where the key's own dictionary word loses slot 0 number 222 at α 10 and 24 at α 50. α 50 keeps more whole words on fully toned keys. On toneless keys, which pool every tone's homophones, it over-merges, and the grid shows the loss there.

**Lesson.** The simulator answers the TL-full-tone question only. A calibration that affects every input variant needs the engine harness on all eight before a choice is made. The P1 plan's "simulate, then confirm" order hid the toneless trade-off until after the choice.

## 2. What α 50 would have changed (recorded, not shipped)

Re-deriving the model price of a 2,000-count word at α 50 moves `CUSTOM_EDGE_COST` from 7,878 to 8,675 milli-nats. It also shortens the selection fade. In `cost.rs` `user_entry_cap_fades_with_the_selection_weight`, a single pick decayed to 5 % of `BOOST_ALPHA` (about 90 days at τ = 30 d) no longer beats the test's two-common-character split: 10.147 > 10.034 nats, crossing at about 5.7 %, about 85 days. The S0 golden moved only on its two custom-edge rows (`taigi` 台語 −6.5589 → −7.2224; `taigikhipoann` 台語齒盤 −6.2236 → −6.8532), with the hanji unchanged. All of this was reverted along with α.

## 3. `final` (read once, α 10)

| Variant | n | Exact % | Segmented % | Mean rank |
|---|---|---|---|---|
| tl_full / poj_full / tl_partial | 82 | 85.4 | 91.5 | 0.68 |
| tps_full | 82 | 79.3 | 84.1 | 0.99 |
| tl_toneless / poj_toneless | 82 | 47.6 | 50.0 | 2.49 |
| tps_toneless | 82 | 47.6 | 50.0 | 2.48 |
| all | 656 | 70.4 | 72.6 | 1.40 |

Simulator, TL full tone, same items: pre-E1 85 % / 88 % → E1 85 % / 90 %. By category: `phrase` (n 67) 84 / 85 → 84 / 88, while `common`, `rare` and `unseen` stay at 100 / 100.

## 4. Seen while measuring (not changed here)

- **Toneless input is the weakest stratum on held-out text.** `final` TL toneless is 47.6 % exact against 70.5 % on `dev` (a 22.9-point gap; TL full tone's gap is 1.0 point). `dev` overlaps the corpus that trains the model (plan §5), so the toneless `dev` figure is optimistic.

## 5. Commands

```
make dict                                  # at each α (dictionary/build/common.py WALKER_ALPHA)
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold resolve
GOLD_OUT=<tsv> cargo test --manifest-path engine/Cargo.toml -p composing --test prod walker_gold -- --include-ignored
GOLD_SPLITS=final GOLD_OUT=<tsv> cargo test … walker_gold -- --include-ignored     # once, α 10
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold simulate --splits calib --alphas 0.5 2 10 50
cd dictionary && PYTHONPATH=. python3 -m tools.walker_gold exposure --alpha 50
```
