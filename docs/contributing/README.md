# Contributing guides

Per-platform style guides and cross-cutting rules. [Which guide for which path](#which-guide-for-which-path) says which to read; [`AGENTS.md`](../../AGENTS.md) lists what else to read before which kind of task. Repository-wide basics (licensing, build, commits) are in [`CONTRIBUTING.md`](../../CONTRIBUTING.md).

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
| [`web-guidelines.md`](web-guidelines.md) | Web version: build, pins, raw-JS JSDoc + `tsc` `checkJs`, what `wasm32` cannot do |

## Which guide for which path

Read a guide once, before your first edit to a file one of its paths matches. Claude Code is pointed at the same guides by the pointers in `.claude/rules/`; `make lint-agent-rule-paths` fails when the two lists differ.

| Guide | Paths |
|---|---|
| [`android-guidelines.md`](android-guidelines.md) | `android/**/*.kt`, `android/**/*.gradle*` |
| [`cross-platform-alignment.md`](cross-platform-alignment.md) | `ios/**/*.swift`, `android/**/*.kt`, `engine/**`, `dictionary/**`, `docs/architecture/behavioral-invariants.md` |
| [`doc-lookup.md`](doc-lookup.md) | `ios/**/*.swift`, `macos/**/*.swift`, `android/**/*.kt` |
| [`i18n.md`](i18n.md) | `i18n/**` |
| [`ios-architecture.md`](ios-architecture.md) | `ios/**/*.swift` |
| [`ios-guidelines.md`](ios-guidelines.md) | `ios/**/*.swift` |
| [`linux-guidelines.md`](linux-guidelines.md) | `linux/**`, `desktop/**` |
| [`phonetics.md`](phonetics.md) | `engine/phonetics/**`, `engine/composing/**`, `engine/lexicon/**`, `engine/protos/**`, `docs/phonetics/taigi-phonetics-reference.md`, `taigi-converter/**`, `dictionary/**`, `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+Phonetics.swift`, `ios/Sources/TaigiKeyboard/Engine/RustEngineBridge+CaseTransform.swift`, `android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/ExternalLookupURLBuilder.kt`, `android/app/src/main/java/com/siansiansu/taigikeyboard/engine/PhoneticsBridge.kt`, `android/app/src/main/java/com/siansiansu/taigikeyboard/engine/CaseTransformBridge.kt` |
| [`rust-best-practices.md`](rust-best-practices.md) | `engine/**/*.rs`, `engine/**/Cargo.toml` |
| [`rust-ffi-safety.md`](rust-ffi-safety.md) | `engine/android-jni/**`, `engine/swift-ffi/**`, `engine/dispatch/**`, `engine/protos/**`, `engine/**/src/api.rs`, `engine/**/src/requests.rs`, `engine/**/src/handle.rs`, `macos/crates/taigi-macos-ffi/**`, `linux/crates/taigi-linux-ffi/**`, `windows/crates/taigi-windows-tsf/**`, `web/crates/taigi-web/**` |
| [`rust-migration-policy.md`](rust-migration-policy.md) | `engine/**`, `docs/engine/migration-inventory.csv`, `ios/Sources/TaigiKeyboard/Engine/**`, `android/app/src/main/java/com/siansiansu/taigikeyboard/engine/**` |
| [`security-rules.md`](security-rules.md) | `ios/**`, `android/**`, `macos/**`, `windows/**`, `linux/**`, `desktop/**`, `engine/**`, `web/**` |
| [`ui-style-guide.md`](ui-style-guide.md) | `ios/Sources/TaigiKeyboard/App/**`, `android/app/src/main/java/com/siansiansu/taigikeyboard/settings/**`, `android/app/src/main/java/com/siansiansu/taigikeyboard/ui/**` |
| [`web-guidelines.md`](web-guidelines.md) | `web/**` |
| [`windows-guidelines.md`](windows-guidelines.md) | `windows/**` |
