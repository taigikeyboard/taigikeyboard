# Docs Authoring Rules

Mandatory rules for writing or editing user-facing documentation in this repo (`README.md`, `CHANGELOG.md`, `content/**`, FAQ, marketing copy).

## Solo maintainer — no public build instructions

This project has ONE maintainer who builds locally. Public docs (`README.md`, in-app content, FAQ) must NOT contain "Local development", "How to build", "How to run" sections.

- Skip `git clone` → `cd` → `./gradlew assembleDebug` flows.
- Skip `xcodebuild` / Xcode setup walkthroughs.
- Skip `cargo build` / Rust toolchain setup.
- Skip dependency installation hints.

**Why**: there are no outside contributors who need these. Public build docs are dead text that ages out, contradicts the actual build flow when it changes, and signals "we accept PRs" when this repo does not.

## Where command lists DO belong

Agent-facing docs keep command lists for the model's use:

- `rules/*.md` — references commands the agent should know about.
- `docs/engine/*.md` — engine-side build / golden / parity test invocations.
- `Makefile` — canonical invocation surface (`make fmt-check`, `make lint`, `make test`, `make build`).
- `CLAUDE.md` "Build & Test" table — quick reference for the AI agent.

This separation is deliberate. User-facing docs answer "what does this project do?"; agent-facing docs answer "how does the assistant operate on it?".

## When the user explicitly asks

If the user asks for a public-facing build guide ("add a Quick Start to README"), do it. The rule above is the default, not an absolute.
