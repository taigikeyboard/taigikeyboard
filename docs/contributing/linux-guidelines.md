# Linux Project Guidelines

Rules for the Linux input method (`linux/`: Fcitx5 addon (primary, C++ over a Rust C ABI)
+ IBus engine (second, pure Rust) + settings window, over `desktop/` + the shared engine). Read before modifying Linux or `desktop/` code. Design
record: `docs/architecture/linux-roadmap.md`.

## Host gates prove compilation

- Linux behaviour is observed only in the dogfood VMs and the `/e2e` run, never on the macOS host. Every PR passes
  `make linux-check` on the macOS host: `cargo test --workspace` in `linux/` (native), `cargo clippy
  --workspace --all-targets -- -D warnings` in `linux/` (native: `zbus` + gtk4-rs + libadwaita
  from Homebrew), a real cross build of the engine for `x86_64-unknown-linux-gnu` via `cargo zigbuild`
  (`mise install`; plain `cargo check` stops at the bundled SQLite's C build),
  `cargo fmt -- --check`, the i18n check. The nightly CI job (`linux-build.yml`, not per PR — run `make linux-check` before a Linux PR) adds the real
  Linux build, the daemon smoke and the `.deb` (`make -C linux deb`). These gates prove compilation and the wire's
  first contract, not behaviour — behaviour is the dogfood run-book's job. Never claim "works
  on Linux".
- Put logic in `taigi-desktop-core` (host-testable; the oracle rule is `windows-guidelines.md`
  § desktop-core is the behaviour oracle) whenever it needs no D-Bus / GTK handle. `taigikeyboard-ibus` and
  `taigikeyboard-settings` are thin shells; `taigi-linux-platform` holds the few
  Linux-specific pure pieces (XDG paths, keysym translation, launcher).
- A change to `desktop/` is a change to all three desktops: `python3 tools/test_select.py --run`
  selects the desktop, Windows, Linux and both macOS gates (the macOS ones behind `make build`);
  `make windows-check`'s gnu cross-clippy compiles the crates through the Windows graph.

## IBus wire discipline

- Serialised shapes come from the ibus source, never from memory: `IBusText`
  `("IBusText", a{sv}, s, v)`, `IBusAttrList` `("IBusAttrList", a{sv}, av)`, `IBusAttribute`
  `("IBusAttribute", a{sv}, uuuu)`, `IBusLookupTable` `("IBusLookupTable", a{sv}, uubbiavav)`,
  `IBusProperty` `("IBusProperty", a{sv}, suvsvbbuvv)`, `IBusPropList` `("IBusPropList",
  a{sv}, av)`. Every nested serialisable is wrapped in a `v`. Each wire type has a unit test
  asserting its `zvariant` signature string — a field added or reordered fails there, not on a
  user's daemon.
- D-Bus method names are PascalCase on the wire (`ProcessKeyEvent`); zbus derives them from
  the snake_case Rust names — do not rename a handler without checking the interface
  introspection in `src/ibusengine.c`.
- Key releases (`IBUS_RELEASE_MASK`, bit 30) and bare modifier presses answer `false`
  untouched. Auto-repeat is invisible on the wire: a toggle chord is latched until another key
  arrives.
- Engine state lives behind one `Mutex`; never `.await` while holding it, never call back into
  the daemon (signal emission) with it held — emit after the lock is dropped, the way Windows
  touches the candidate window only after the edit session returned.
- Every D-Bus method body runs under `catch_unwind`; a panic answers `false` / `()` and logs.
  The daemon respawns a dead engine and the user loses the composition.
- No `libibus`, no GObject in the Rust crates of `linux/`. The settings window
  (`taigikeyboard-settings`, GTK 4 + libadwaita) is the one crate that links a system C library;
  `taigikeyboard-ibus` and `taigi-linux-ffi` must build on the macOS host and cross-build for
  `x86_64-unknown-linux-gnu` (`linux/Makefile` `check-target`). The ONE crate allowed `unsafe` is
  `taigi-linux-ffi` (the C ABI the Fcitx5 addon calls), one `// SAFETY:` line per call,
  `catch_unwind` at every exported function. The Fcitx5 addon itself (`linux/fcitx5/`, C++) is
  built only in the VM and on the Ubuntu CI job; it composes nothing — every effect comes back
  from the FFI as the same `Emit` list the IBus shell replays.
- Two shells, one contract: a behaviour that differs between the Fcitx5 and IBus shells is a shell
  bug. Fix it in `taigi-desktop-core::session` or `taigi-linux-core`, never in one shell.

## Settings window (GTK 4 + libadwaita)

- Native widgets only (maintainer: "use native UI components for the settings UI"): `adw::PreferencesPage` / `Group` /
  `ActionRow` / `SwitchRow` / `ComboRow` / `EntryRow` / `ExpanderRow`, `adw::AlertDialog`,
  `adw::Banner`, `adw::Toast`, `gtk::ColumnView`, `gtk::FileDialog`. No custom-drawn cards.
- Pin the `gtk4` / `libadwaita` crate versions to what the Homebrew and Ubuntu libraries
  provide (`gtk4 = "0.11"`, `libadwaita = "0.9"`; feature `v1_5`); a bump is its own round,
  built on both.
- Each pane is mounted headlessly in a test (offscreen window) — a new pane belongs in that
  test, the way a Windows pane belongs in `pane_planning`.
- The recorder reads `gtk::EventControllerKey` on the focused row and decides through
  `taigi-desktop-core::keys::evaluate_press`; no global grab.

## Data + install

- `settings.json` = `$XDG_CONFIG_HOME/taigikeyboard/`; databases = `$XDG_DATA_HOME/taigikeyboard/`;
  dictionaries = `${prefix}/share/taigikeyboard/dictionaries` (`TAIGIKEYBOARD_PREFIX` at build,
  `TAIGIKEYBOARD_DATA_DIR` at runtime for a dev tree). Never the working directory.
- No update check on Linux, manual or automatic, and no build flag for one — the distribution's package manager updates an input method (`linux-release.md` § No in-app update). Never link `taigi-desktop-update`; the `update*` keys stay unwritten. Do not re-propose.
- Release / package / tag actions are the maintainer's call.
