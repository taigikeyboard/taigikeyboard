# Corpus recount after plain-Hanji normalization

Follow-up to issue #929, stacked on PR #931 at
`f892f44e5e7d1674f0ab304b69e5bb67b9bb008d`.

## Inputs and method

The recount used the repository's pinned submodules without changing their revisions:

| Input | Revision |
| --- | --- |
| `corpus/taigi-corpus` | `e2176c7e3be6f8b7a6c9a0da1f45d0849a5aec2a` |
| `corpus/taigi-typing` | `3bd508ee013c8172afb6e0f50de829859da84d06` |
| `taigi-converter` | `65760da2e762f0ce37ff4da33fad3b9aafcf5be1` |

Rebuilding `dictionary/output/dictionary.csv` from the committed source CSVs
produced an identical file before counting. All nine sources in
`dictionary/build/corpus_bigrams.py::SOURCES` were counted with the existing
two-pass algorithm, sentence cap of three and minimum bigram count of two.
The pass-one tie-break model had `N=3155646`, `V=167493`, `alpha=10.0`.

From `dictionary/`, the reproduction commands are:

```sh
PYTHONPATH=. python3 -m build.corpus_bigrams
bash build.sh
python3 -m pytest tests -q
```

All aggregate build stages and `build/deploy.sh` ran. For this jj workspace,
the report-only release-history reads in `build.version_snapshot` used
`jj tag list` and `jj file show` in place of Git reads.

## Root cause and recovered counts

Before PR #931, `dictionary/build/corpus_bigrams.py:239` at `adcb61db`
called `hanji_with_khinsiann` while aligning pure-Hanji words. This inserted
`--` before the `(hanji, tl_num)` whitelist lookup, rejecting neutral-tone
words whose dictionary Hanji was plain. PR #931 removed the insertion at
the current line 223, but mechanically stripped the committed count tables;
that could not recover rejected occurrences or their adjacent pairs.

The MOE example containing `ài ē-kì--tit` reproduces the old failure:
`愛/ai3` was followed by a chain break. With the parent fix it is followed by
`會記得/e7ki3tit4`. Across the full corpus that reading's count rises from
115 to 119. Other recovered counts include:

| Word or pair | Previous count | Recount |
| --- | ---: | ---: |
| `出來/tshut4lai5` | 1,568 | 1,994 |
| `轉來/tng2lai5` | 712 | 994 |
| `袂/be7 → 出來/tshut4lai5` | 21 | 64 |

## Output changes

| Measure | Parent #931 | Recount |
| --- | ---: | ---: |
| Unigram rows | 51,795 | 52,053 |
| Unigram occurrences, including held-out source | 3,629,545 | 3,635,703 |
| Bigram rows | 283,246 | 284,172 |
| Bigram occurrences, including held-out source | 2,127,141 | 2,131,311 |
| Walker training tokens, excluding `taigi_typing` | 3,627,708 | 3,633,858 |
| Dictionary records with a training count | 52,117 | 52,397 |

Comparison uses the dictionary records' model identity `(hanji, tl_num.lower())`.
The parent bigram table had 90 extra rows for alternate spellings of the same
pair; the runtime loader sums those rows. The recount emits one row per model
pair. After summing parent collisions, it adds 1,053 pairs and removes 37.
Unigrams gain 260 identities and lose two. Among identities present in both
tables, 659 unigram counts increase and 53 decrease; 873 bigram counts increase
and 131 decrease. This is a fresh two-pass recount, which also recomputes the
subsegmentation tie-break model, rather than an additive patch to old counts.

Both tables passed source-column sum checks and dictionary-identity validation;
the recount has no duplicate model identities. `taigi_typing` remains held out
by the walker model. The dictionary vocabulary remains 168,197 model words over
168,716 records. The merged CSV, word-key set and both FST artifacts are identical
to the parent; the dictionary and association binaries carry the refreshed counts.

## Validation

- Dictionary build: binary verification, POJ integrity and known-key gates passed.
- `pytest dictionary/tests`: 255 tests and 19 subtests passed, including
  `test_walker_gold.py`, `test_walker_lm.py` and corpus-counting tests.
- `make test`: 1,715 engine tests passed, five existing tests ignored.
- Dispatch tests with `e2e-trace`, selected-crate Clippy with warnings denied,
  and engine formatting passed on the installed stable toolchain (Rust 1.100.0).
- The production bigram test initially failed on the old continuation totals
  for `有/u7`. Direct `walker_lm.load_bigrams` reads confirmed the new
  `(pair_count, total, distinct)` for `有/u7 → 的/e5` is `(411, 48806, 3381)`;
  the test expectation now matches the recount.
- Platform app builds and device typing were not run.

## Output SHA-256

```text
f57cd2998e88e8493142fd9bcf9b82e103c5be28483640f8924f13097dc1bf6f  word_unigrams.tsv
63a429f524d44a5f5ce648072f0a801510d1aaa09ededabc04982e3619468a4e  word_bigrams.tsv
817cc23c2471a3b5a63e26ffcb60a1b4329b59dda2295ec51abbf3bd264a42ca  dictionary.bin
d4d57b0dc28ba981d48b5952aca3bc2c8b35d4491d532bbeb427c175750084e0  association.bin
```
