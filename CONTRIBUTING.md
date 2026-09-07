# Contributing

Taigi Keyboard is a Taiwanese input method for iOS, Android,
macOS, and Windows, built on a shared Rust engine.

Write to the project in Taiwanese, Mandarin, or English — whichever you are
most comfortable in. Code, comments, and documentation stay in English.

## Licensing of contributions

By submitting a contribution you agree that it is licensed under the
[Apache License, Version 2.0](LICENSE), the same licence as the project. This
is the default in Apache-2.0 §5; there is no separate CLA to sign.

Two things that are **not** covered by that:

- **Dictionary data.** Do not submit dictionary entries copied from a source
  whose licence you have not checked. Every source is inventoried in
  [`dictionary/LICENSE`](dictionary/LICENSE), and several rows there are
  unresolved. Adding data with unknown terms makes that worse. New entries you
  wrote yourself belong in `dictionary/supplementary/dev/`.
- **Third-party code.** If you vendor something, keep its original copyright
  header and licence, and add a row to
  [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md).

## Before you open a pull request

Read [`CLAUDE.md`](CLAUDE.md) — despite the name, it is the working
description of this repository's layout, conventions, and per-platform rules,
and the per-platform style guides in `.claude/rules/` are the real style
guides.

Behaviour that four platforms must agree on is specified in
[`docs/architecture/behavioral-invariants.md`](docs/architecture/behavioral-invariants.md).
A change to shared behaviour is a change to all four platforms, or it is a
regression on three of them.

## Build and test

The `Makefile` at the repository root is the canonical entry point; the
per-platform build and test invocations are tabulated in
[`CLAUDE.md`](CLAUDE.md) § Build & Test. Those are kept current — do not
work from a copy of them pasted anywhere else.

One thing worth knowing before you run anything: **if your change touches
`engine/` or `dictionary/`, regenerate the platform artifacts before testing
iOS or Android** (`make dict` if the dictionary changed, then `make build`).
Both platforms link against pre-built binaries, so a stale artifact gives
passing tests against the *old* engine. That is the single most common source
of "tests passed but the keyboard misbehaves on device".

## Secrets

Run `make hooks` once per clone. It points `core.hooksPath` at `.githooks`, whose
`pre-commit` scans staged changes with gitleaks and refuses a commit that carries
a credential. A clone that never ran it has no local gate at all, and
`git commit --no-verify` skips it in one that did — which is why the same scan
also runs in CI on every pull request.

Note that `core.hooksPath` is repository-level and shared by every worktree, so
do not run `make hooks` while another checkout of this repository is mid-task.

`make scan-secrets` scans everything since the last recorded clean full-history
pass; `.gitleaks-scanned` explains when that recording stops being valid and has
to be redone with `make scan-secrets-full`.

## Phonetics

Never infer a TL / POJ / TPS rule. Read
[`knowledge/taigi-phonetics-reference.md`](knowledge/taigi-phonetics-reference.md)
and check against the `taigi-converter/` submodule, which is the canonical
converter. A plausible-looking rule that is wrong will silently corrupt the
dictionary index.

Note that a Taiwanese word is identified by the **pair** (漢字, romanization),
never by either alone — 重/tîng and 重/tāng are different words, and so are two
different 漢字 sharing a reading. Anything that deduplicates, looks up, ranks,
or merges entries must key on the pair.

## Commits and pull requests

- Conventional Commits: `type(scope): subject`, e.g. `fix(engine): …`
- One pull request per logical change
- Explain *why* in the body; the diff already shows *what*
- Say which platforms you built and tested

## Reporting bugs

Open an issue with: platform and version, what you typed, what appeared, and
what you expected instead. For an input-method bug the exact keystroke
sequence is the whole report — `ㄍㄠ` then space then `ㄉㄞ` is useful,
"tones are wrong" is not.

Security issues go to <info@taigikeyboard.tw> instead — see
[`SECURITY.md`](SECURITY.md).
