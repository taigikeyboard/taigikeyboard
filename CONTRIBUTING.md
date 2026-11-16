# Contributing

TaigiKeyboard is a Taiwanese input method for iOS, Android,
macOS, Windows, and Linux, built on a shared Rust engine.

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

Read [`AGENTS.md`](AGENTS.md) — the working description of this repository's
layout, principles and build gates, written for coding agents and humans
alike. The style guide for each platform, and the cross-cutting rules
(phonetics, i18n, cross-platform alignment, known pitfalls), are in
[`docs/contributing/`](docs/contributing/README.md).

Behaviour that the platforms must agree on is specified in
[`docs/architecture/behavioral-invariants.md`](docs/architecture/behavioral-invariants.md).
A change to shared behaviour is a change to every platform that has it, or it
is a regression on the others.

## Build and test

Start with [`docs/BUILDING.md`](docs/BUILDING.md): prerequisites, a pinned
tool list (`mise.toml`), and the build and test command for each platform.
The `Makefile` at the repository root is the canonical entry point; `make help`
lists its targets.

If your change touches `engine/`, `desktop/`, `macos/crates/` or `dictionary/`,
rebuild the native artifacts before testing iOS, Android or macOS — see
[`docs/BUILDING.md`](docs/BUILDING.md) §5. A stale artifact gives passing tests
against the *old* engine.

## Secrets

Run `make hooks` once per clone. It points `core.hooksPath` at `.githooks`, whose
`pre-commit` scans staged changes with gitleaks and refuses a commit that carries
a credential; `commit-msg` checks the message for personal data (below). A clone that never ran it has no local gate at all, and
`git commit --no-verify` skips it in one that did — which is why the same scan
also runs in CI on every pull request.

Note that `core.hooksPath` is repository-level and shared by every worktree, so
do not run `make hooks` while another checkout of this repository is mid-task.

`make scan-secrets` scans everything since the last recorded clean full-history
pass; `.gitleaks-scanned` explains when that recording stops being valid and has
to be redone with `make scan-secrets-full`.

On GitHub, secret scanning and push protection are enabled for this repository:
a push carrying a recognised provider credential is rejected before it lands.
Custom (non-provider) patterns need a paid GitHub Secret Protection licence and
are off; gitleaks covers generic secrets instead.

Personal data is treated like a credential. Do not put an email address, an
account / project / server ID, or a private mailbox or chat layout into any
tracked file, commit message or pull request — describe the person or role
instead. gitleaks' `personal-email` rule (`.gitleaks.toml`) and the maintainer's
private denylist (`tools/secret-scan/private-denylist-scan.sh`) gate commits; the
denylist also reads the commit message (`.githooks/commit-msg`) and the title and
inline body of a pull request opened through Claude Code. Neither sees an issue,
or a pull request opened or edited any other way, so check those by hand. Tooling bound
to a personal account belongs outside this repository, not under `.claude/`.

## Phonetics

Never infer a TL / POJ / TPS rule, and key every dedupe / lookup / ranking on
the (Hanji, romanization) pair — see
[`docs/contributing/phonetics.md`](docs/contributing/phonetics.md).

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
