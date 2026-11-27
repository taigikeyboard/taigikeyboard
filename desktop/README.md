# Desktop-shared crates

The pure Rust the macOS (`../macos`, IMKit, through `macos/crates/taigi-macos-ffi`),
Windows (`../windows`, TSF) and Linux (`../linux`, Fcitx5 + IBus) input methods
link, over the shared engine in `../engine` — `taigi-desktop-core` on all three,
`-storage` on all three (font library on macOS + Windows; settings file + data
directory on Windows + Linux), `-update` on Windows. One workspace, three
crates, no OS handle anywhere — everything here builds and tests natively on
the maintainer's Mac.

| Crate | Role |
|---|---|
| `crates/taigi-desktop-core` | Settings model + revision, engine bridge (prost envelope → `dispatch`), composing orchestration (the macOS `ComposingManager` port), key classifier + shortcuts, candidate geometry, symbol table, generated UI strings. `unsafe_code = forbid`, no C deps of its own: it sees the engine only through `dispatch` + `protos` (the user-data stores through `dispatch`'s ops, `engine::user_data`). |
| `crates/taigi-desktop-update` | The update check the Windows settings window runs (Linux has none — the package manager updates it): manifest wire format + `DottedVersion`, check outcome and what it leaves in `settings.json`, `ureq` HTTPS transport (native TLS, OS roots). The only crate here with a network stack; the schedule is `taigi-desktop-core::settings::update_schedule` so the input-method processes never link it. |
| `crates/taigi-desktop-storage` | The `settings.json` file store (atomic replace, revision), user fonts, the data directory. The user-data stores are the engine's (`../engine/userdata`), reached through `taigi-desktop-core::engine::user_data`. |

Behaviour oracle for the three desktops: [`../docs/contributing/windows-guidelines.md`](../docs/contributing/windows-guidelines.md)
§ desktop-core is the behaviour oracle.

```sh
make check   # i18n check + tests + clippy + fmt --check (from desktop/)
```
