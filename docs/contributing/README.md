# Contributing guides

Per-platform style guides and cross-cutting rules. Read the one that covers the files you change; [`AGENTS.md`](../../AGENTS.md) lists which to read before what. Repository-wide basics (licensing, build, commits) are in [`CONTRIBUTING.md`](../../CONTRIBUTING.md).

| Guide | Covers |
|---|---|
| [`known-pitfalls.md`](known-pitfalls.md) | Rules distilled from past incidents — read once before your first change |
| [`cross-platform-alignment.md`](cross-platform-alignment.md) | iOS ↔ Android contract: refactor freeze + parity tier, shared-core criteria, where each concept lives, invariants, engine non-goals |
| [`phonetics.md`](phonetics.md) | TL / POJ / TPS schema and canonical-form work — authoritative sources only |
| [`i18n.md`](i18n.md) | Shared `i18n/*.json` keys |
| [`doc-lookup.md`](doc-lookup.md) | Verifying a framework / OS API against current docs before calling it |
| [`security-rules.md`](security-rules.md) | Logging, SQL binding, exported components — all platforms |
| [`rust-best-practices.md`](rust-best-practices.md) | Engine workspace, errors, crates, tests |
| [`rust-ffi-safety.md`](rust-ffi-safety.md) | Every FFI boundary (Swift / JNI / dispatch, Linux C ABI, Windows COM): panic, thread, error, logging, opaque handles, test contract |
| [`rust-migration-policy.md`](rust-migration-policy.md) | Moving platform logic into the engine |
| [`ios-guidelines.md`](ios-guidelines.md) | KeyboardKit lookup, extension memory, user data + Full Access, Xcode project files |
| [`ios-architecture.md`](ios-architecture.md) | iOS layers, Engine-layer imports, settings injection, shared-core marker |
| [`ui-style-guide.md`](ui-style-guide.md) | App / settings UI, both mobile platforms |
| [`android-guidelines.md`](android-guidelines.md) | Shared-core marker, DI + lifecycle, IME host facts, DataStore settings, user-data SQLite, Gradle |
| [`windows-guidelines.md`](windows-guidelines.md) | Windows TSF |
| [`linux-guidelines.md`](linux-guidelines.md) | Linux Fcitx5 / IBus and `desktop/` |
| [`web-guidelines.md`](web-guidelines.md) | Web version: build, pins, what `wasm32` cannot do |
