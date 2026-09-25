# Linux Desktop IME — Roadmap

> **Type**: Planning (design record + PR table; becomes Reference once shipped)
> **Keywords**: `linux`, `IBus`, `D-Bus`, `zbus`, `GTK4`, `libadwaita`, `desktop`, `engine reuse`, `macOS parity`, `Windows parity`
> **Status**: **implemented — PR0–PR9 all MERGED 2026-09-23 (#140–#151); first-machine dogfood S74 pending (no Linux machine yet)**. History: PR0 2026-09-22; **2026-09-23 USER decision: Fcitx5 is the primary frontend, the IBus engine (PR3) is kept as the second** (「選項 1，Fcitx5 主、IBus 保留，go」) — a Linux VM for dogfood exists from here on, which removes the one reason IBus was chosen first (§ L1). Authored WITHOUT a Linux machine, the way the Windows platform was (`windows-roadmap.md` § W13); verified on the macOS host + a GitHub-hosted Ubuntu runner, dogfooded later.
> **Session memory**: project memory `project_linux_ime.md` (Claude auto-memory)
> **Siblings**: `macos-roadmap.md` (behaviour oracle), `windows-roadmap.md` (the blind-authoring precedent and the crates this platform reuses)

---

## Goal

Add Linux as the fifth platform (third desktop platform beside macOS and Windows). USER 2026-09-22 (verbatim):

> design and implement linux desktop, the design should be consist with windows and macos … 設定選單UI使用原生UI元件

Three consequences shape every decision below:

1. **macOS stays the behaviour oracle; Windows is the code oracle.** Every observable
   desktop behaviour (key table, modes, auto-space, full-width punctuation, settings keys
   and defaults, storage schemas, CSV, shortcuts) already exists twice — as Swift in
   `macos/` and as host-testable Rust in `desktop/crates/taigi-desktop-core` +
   `-storage`. Linux reuses the Rust verbatim (§ L2); only the shell is new. Deltas are
   named with the Windows five-way classification (**identical semantics** ·
   **platform-adapted presentation** · **unsupported host capability** ·
   **intentionally not in this slice** · **unverified until Linux dogfood**), per
   `.claude/rules/cross-platform-alignment.md` §3.
2. **Native widgets for the settings window** (USER). On Linux "native" is the desktop's
   toolkit: GTK 4 + libadwaita (§ L3), the toolkit GNOME Settings, `ibus-setup` and the
   GNOME IME preference dialogs are built with.
3. **No Linux machine at authoring time.** Every PR passes `make linux-check` on the macOS
   host (§ L12) and the Ubuntu CI job; first-run breakage on a real desktop is the dogfood
   run-book's job (§ Dogfood run-book). Never claim "works on Linux".

**Hard constraints**: release / tag / package actions stay USER-gated. TL + POJ only, no
TPS (same as macOS and Windows). Shared-surface changes (proto, i18n, the crate move,
release tooling) are listed in § Shared-surface register.

**Codex**: the pre-implementation ANALYSIS-ONLY consult (2026-09-22) was refused by the
Codex usage limit (resets 2026-09-23); the decisions below are the author's, grounded in
the sources cited, and are flagged `Codex: skipped (quota)` in the PR table. A later
retro review is welcome and its verdicts belong in this document.

## Architecture

Revised 2026-09-23 (Fcitx5 primary): everything below the shells is shared. The Fcitx5 shell
is `linux/fcitx5/libtaigikeyboard.so` (C++ `InputMethodEngineV3` over the `taigi-linux-ffi` C ABI
over `taigi-linux-core`); the IBus shell is `ibus-engine-taigikeyboard` (zbus over the same
`taigi-linux-core`). The diagram keeps the IBus process as drawn for PR3; the Fcitx5 addon
replaces its top box with `fcitx5` (in-process addon, `keyEvent` → FFI → `Emit`s → input panel).

```
Application (GTK / Qt / Chromium / Electron / terminal) ── IBus client module (im-module / Wayland text-input)
        ▲ CommitText / UpdatePreeditText            │ ProcessKeyEvent(keyval, keycode, state)
        │                                           ▼
   ibus-daemon (session bus of its own; renders preedit, lookup table, panel indicator + menu)
        ▲ D-Bus (private bus address from ~/.config/ibus/bus/*)   │ CreateEngine("taigikeyboard")
┌───────┴────────────────────────────────────────────────────────▼────────────────────────┐
│ ibus-engine-taigikeyboard  (crate taigikeyboard-ibus, bin; pure Rust over `zbus`, no libibus)│
│  bus connection + Hello + RequestName("org.freedesktop.IBus.TaigiKeyboard")            │
│  /org/freedesktop/IBus/Factory  : org.freedesktop.IBus.Factory (CreateEngine → path)    │
│  /org/freedesktop/IBus/Engine/N : org.freedesktop.IBus.Engine (one object per client   │
│    context; ProcessKeyEvent → KeyEventSnapshot → ComposingKeyIntent → ComposingManager  │
│    → effects rendered as UpdatePreeditText / CommitText; candidates as UpdateLookupTable │
│    with slot-key labels; panel menu as RegisterProperties; settings live reload)         │
│  wire types (IBusText / IBusAttrList / IBusLookupTable / IBusProperty / IBusPropList)   │
│    hand-serialised to the `(sa{sv}…)` GVariant shapes ibus itself emits (§ L1)          │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│ taigi-linux-platform (lib, host-testable): XDG paths, install prefix, keysym → snapshot  │
│   translation, settings launcher, open-URL                                              │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│ desktop/ workspace (moved from windows/, § L2): taigi-desktop-core (pure; settings model,│
│   engine bridge, composing orchestration, key intent table, shortcuts, strings) ·        │
│   taigi-desktop-storage (rusqlite freq v2 / assoc v6 / custom v4 / learned v1,           │
│   settings.json file store + revision, CSV)                                             │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│ engine/ crates by path (dispatch, protos, composing, lexicon, phonetics, nextword)       │
└─────────────────────────────────────────────────────────────────────────────────────────┘
        │ spawn `taigikeyboard-settings --pane …` (also ibus-setup's "Preferences" via <setup>)
        ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│ taigikeyboard-settings (crate taigikeyboard-settings, bin; gtk4-rs + libadwaita)         │
│  adw::ApplicationWindow + NavigationSplitView sidebar: 一般 / 外觀 / 快捷鍵 / 詞庫來源 /  │
│  自訂詞庫 / 關於 (+ unlisted 辭典搜尋); PreferencesPage / PreferencesGroup / ActionRow /  │
│  SwitchRow / ComboRow; AlertDialog; Banner; ColumnView table; gtk::FileDialog CSV        │
└─────────────────────────────────────────────────────────────────────────────────────────┘
Runtime data: $XDG_CONFIG_HOME/taigikeyboard/settings.json ·
$XDG_DATA_HOME/taigikeyboard/{user_frequency.db, user_association.db, custom_dictionary.db,
learned_phrases.db}. Install: ${prefix}/libexec/ibus-engine-taigikeyboard,
${prefix}/bin/taigikeyboard-settings, ${prefix}/share/ibus/component/taigikeyboard.xml,
${prefix}/share/taigikeyboard/dictionaries/*, ${prefix}/share/applications/…desktop, icons.
```

## Design decisions (L1–L13, grounded in code and in the ibus source)

Citations: `windows/…` and `macos/…` are this repository at PR0; `ibus src/*.c` is
github.com/ibus/ibus `main` as fetched 2026-09-22 (the introspection XML and the
`serialize` functions were read, not remembered).

- **L1 Frameworks = Fcitx5 (primary) + IBus (second), one Rust core, two thin shells.**
  **Revised 2026-09-23** (USER: 「我認為使用Fcitx5比較好，我可以安裝linux vm測試」 → 「選項 1，
  Fcitx5 主、IBus 保留，go」). The original choice of IBus first had ONE real reason: no Linux
  machine, so only a pure-D-Bus engine could be compiled and checked from the Mac; with a
  VM that reason is gone, and Fcitx5 is where Taiwanese / CJK Linux users are (fcitx5-chewing,
  fcitx5-rime, KDE's default), has the most complete Wayland story, draws its own themed
  candidate panel, and offers real global hotkeys. Mainstream engines ship both (`ibus-rime`
  + `fcitx5-rime`, `ibus-mozc` + `fcitx5-mozc`, `ibus-chewing` + `fcitx5-chewing`), which
  is the shape adopted:
  - **`taigi-linux-core`** (Rust lib, pure): the framework-independent half of what PR3 wrote
    inside the IBus crate — `Runtime` (settings, stores, lexicon, coordinator), the key path
    (`session.rs`: snapshot → intent → manager → `Emit`s), the `Emit` effect model
    (preedit / commit / delete-surrounding / lookup table), `LookupSelection`. Both shells
    replay the same `Emit` list; nothing composes in a shell.
  - **`taigi-linux-ffi`** (Rust `staticlib`, the ONE Linux crate allowed `unsafe`, `// SAFETY:`
    per call as `rust-ffi-safety.md` §3): a C ABI over `taigi-linux-core` — opaque runtime /
    engine handles, `taigi_engine_key(keysym, keycode, states) → reply`, accessor functions
    over the reply's emits (kind, text, caret, table rows / labels / cursor / orientation), a
    hand-written `taigikeyboard.h` pinned by a test that compiles it. `catch_unwind` at every
    entry (`docs/engine/ffi-safety.md` §2), never a panic across the boundary.
  - **`linux/fcitx5/`** — the Fcitx5 addon, C++ (the framework's addons are in-process C++;
    there is no Rust binding worth pinning): `FCITX_ADDON_FACTORY_V2`, an
    `InputMethodEngineV3` whose `keyEvent` hands `rawKey().sym()` / keycode / states to the
    FFI and replays the reply — `inputPanel().setClientPreedit(Text)` with
    `TextFormatFlag::Underline` + `setCursor`, `commitString`, `deleteSurroundingText`, a
    `CommonCandidateList` (`setLabels`, `setPageSize`, `setLayoutHint`, `setGlobalCursorIndex`,
    `CandidateWord::select` = highlight + commit through the FFI) — then `updatePreedit()` +
    `updateUserInterface(InputPanel)`; `reset` / `deactivate` end the session. What is on
    screen is written first wherever IBus writes it under `PREEDIT_COMMIT`: Fcitx5 writes the
    client preedit on focus loss (5.1.7 `instance.cpp:1037`) and fcitx5-gtk before a Reset
    (`fcitx_im_context_reset`), so the shell writes it only on a switch to another input
    method (`deactivate` with `InputContextSwitchInputMethod` — ibus
    `bus_input_context_unset_engine`); `activate` refreshes the status area (menu actions, § L6);
    `subModeLabelImpl` = the mode symbol, `subMode` = the full mode label (§ L6); the
    field's capability and password flags are read per key. `Configurable=True` with one
    `SubConfigOption` whose `setSubConfig` opens the settings window — the configure button
    of `fcitx5-configtool` / the KDE page, as the IBus component's `<setup>` is GNOME's.
    Built by CMake against `Fcitx5Core` (the pre-5.1.12 `add_library(MODULE)` + empty
    `PREFIX` shape of `fcitx5-rime` 5.1.8 `src/CMakeLists.txt` — Ubuntu 24.04 ships fcitx5
    5.1.7, which has neither `add_fcitx5_addon` nor `FCITX_ADDON_FACTORY_V2`), installed to
    `${libdir}/fcitx5/libtaigikeyboard.so` (the loader resolves `Library=export:libtaigikeyboard`
    to that name) + `${datadir}/fcitx5/{inputmethod,addon}/taigikeyboard.conf`. **Not host-buildable** — the
    VM and the Ubuntu CI job (`fcitx5-modules-dev`, `extra-cmake-modules`) own it; the Mac
    gates the Rust half (`taigi-linux-core` tests, `taigi-linux-ffi` cross build via zig).
  - **`taigikeyboard-ibus`** stays as PR3 built it, rebased onto `taigi-linux-core`: the D-Bus
    wire (`bus`, `wire`, `factory`, `engine`) is all that remains in it. GNOME users get it
    from the same package.
  The IBus wire facts below stay authoritative for that shell. What the two shells must
  agree on is the `Emit` contract, so every divergence between them is a shell bug, not a
  design choice.
  **The IBus shell (PR3, as built)**: engine in pure Rust over D-Bus (`zbus` 5) — no `libibus`,
  no GObject, no C toolchain on the host. IBus is GNOME's default framework and the one Ubuntu / Fedora /
  Debian desktops ship enabled; mainstream CJK IMEs ship an IBus engine first (rime
  `ibus-rime`, mozc `ibus-mozc`, libchewing `ibus-chewing`). What the wire needs was
  taken from the ibus source:
  - `org.freedesktop.IBus.Factory` at `/org/freedesktop/IBus/Factory`:
    `CreateEngine(s name) → o object_path` (`src/ibusfactory.c` introspection XML;
    engine paths are `/org/freedesktop/IBus/Engine/<n>`, `ibus_factory_real_create_engine`).
  - `org.freedesktop.IBus.Engine` methods: `ProcessKeyEvent(u keyval, u keycode, u state) → b`,
    `SetCursorLocation(iiii)`, `SetCapabilities(u)`, `PropertyActivate(s, u)`,
    `PropertyShow(s)`, `PropertyHide(s)`, `CandidateClicked(uuu)`, `FocusIn` /
    `FocusInId(ss)` / `FocusOut` / `FocusOutId(s)`, `Reset`, `Enable`, `Disable`, `PageUp` /
    `PageDown` / `CursorUp` / `CursorDown`, `SetSurroundingText(vuu)`,
    `ProcessHandWritingEvent(ad)`, `CancelHandWriting(u)`, `PanelExtensionReceived(v)`,
    `PanelExtensionRegisterKeys(v)`; signals `CommitText(v)`, `UpdatePreeditText(v, u
    cursor_pos, b visible, u mode)`, `UpdateAuxiliaryText(v, b)`, `UpdateLookupTable(v, b)`,
    `RegisterProperties(v)`, `UpdateProperty(v)`, `ForwardKeyEvent(uuu)`; properties
    `ContentType (uu)` write, `FocusId (b)` read, `ActiveSurroundingText (b)` read
    (`src/ibusengine.c` introspection XML). `FocusId` answers `false`, so the daemon uses
    the plain `FocusIn` / `FocusOut` pair.
  - Serialisation: every `IBusSerializable` is a tuple `(s type_name, a{sv} attachments,
    …fields)` (`src/ibusserializable.c:277`). `IBusText` = `("IBusText", a{sv}, s text,
    v attrs)`; `IBusAttrList` = `("IBusAttrList", a{sv}, av)`; `IBusAttribute` =
    `("IBusAttribute", a{sv}, u type, u value, u start, u end)` with
    `IBUS_ATTR_TYPE_UNDERLINE = 1`, `IBUS_ATTR_UNDERLINE_SINGLE = 1`
    (`src/ibusattribute.h:78-99`); `IBusLookupTable` = `("IBusLookupTable", a{sv},
    u page_size, u cursor_pos, b cursor_visible, b round, i orientation, av candidates,
    av labels)` with `IBUS_ORIENTATION_HORIZONTAL = 0`, `VERTICAL = 1`
    (`src/ibuslookuptable.c`, `src/ibustypes.h:151`); `IBusProperty` = `("IBusProperty",
    a{sv}, s key, u type, v label, s icon, v tooltip, b sensitive, b visible, u state,
    v sub_props, v symbol)` with `PROP_TYPE_{NORMAL 0, TOGGLE 1, RADIO 2, MENU 3,
    SEPARATOR 4}`, `PROP_STATE_{UNCHECKED 0, CHECKED 1}` (`src/ibusproperty.c`,
    `src/ibusproperty.h:79-111`); `IBusPropList` = `("IBusPropList", a{sv}, av)`.
  - Modifier bits: `SHIFT 1<<0`, `LOCK 1<<1`, `CONTROL 1<<2`, `MOD1 (Alt) 1<<3`, `MOD4
    1<<6`, `SUPER 1<<26`, `RELEASE 1<<30` (`src/ibustypes.h:70-97`); capabilities
    `PREEDIT_TEXT 1<<0`, `AUXILIARY_TEXT 1<<1`, `LOOKUP_TABLE 1<<2`, `FOCUS 1<<3`,
    `PROPERTY 1<<4`, `SURROUNDING_TEXT 1<<5` (`:119-127`); preedit mode
    `IBUS_ENGINE_PREEDIT_CLEAR = 0`, `COMMIT = 1` (`:138-139`).
  - Bus address: `$IBUS_ADDRESS` if set, else the `IBUS_ADDRESS=` line of
    `$XDG_CONFIG_HOME/ibus/bus/<machine-id>-<hostname>-<display>` (`<machine-id>` from
    `/var/lib/dbus/machine-id` or `/etc/machine-id`; Wayland: `<hostname>` = `unix`,
    `<display>` = `$WAYLAND_DISPLAY`; X11: `$DISPLAY` split at `:` and `.`; the file is
    ignored when its `IBUS_DAEMON_PID` is dead) — `src/ibusshare.c:130-285`. The bus is
    a message bus: GDBus connects with `MESSAGE_BUS_CONNECTION` (Hello is implicit,
    `src/ibusbus.c:508-555`, `:1271`) and `RequestName` goes to
    `org.freedesktop.DBus` (`:1283-1296`). `zbus::connection::Builder::address(…)`
    performs the same Hello; the engine then `request_name`s
    `org.freedesktop.IBus.TaigiKeyboard`.
  - Registration is by component XML at `${prefix}/share/ibus/component/taigikeyboard.xml`
    (`<component>` with `<name>org.freedesktop.IBus.TaigiKeyboard</name>`,
    `<exec>${prefix}/libexec/ibus-engine-taigikeyboard --ibus</exec>`, one `<engine>` named
    `taigikeyboard`, `<language>nan</language>`, `<layout>us</layout>`, `<symbol>台</symbol>`,
    `<setup>${prefix}/bin/taigikeyboard-settings</setup>`, `<rank>0</rank>`). `ibus-daemon`
    reads the directory at start (or after `ibus write-cache`), spawns the exec on demand,
    and the daemon — not the engine — owns activation, so no `RegisterComponent` call.
  (Superseded 2026-09-23: the "Fcitx5 not in this slice" line that stood here — the seam it
  named, `taigikeyboard-ibus` (wire) over `taigi-linux-platform` + `taigi-desktop-core`, is
  exactly the `taigi-linux-core` split above.) **Risk (blind)**: the exact `a{sv}` attachment shape
  and the `v`-wrapping of nested serialisables are the two places a byte-level mistake
  would show only on a real daemon; both are pinned by unit tests against the signature
  strings `(sa{sv}sv)` etc. and by the CI smoke (§ L12).
- **L2 Crate sharing = a `desktop/` Cargo workspace.** `windows/crates/taigi-windows-core`
  → `desktop/crates/taigi-desktop-core`, `taigi-windows-storage` →
  `taigi-desktop-storage`, package + module names renamed mechanically, consumed by both
  `windows/` and `linux/` by path — the same relationship both have to `engine/`. Core
  Principle #5 (direction-first): a Linux workspace that depends on
  `../windows/crates/…` would be the smaller diff and the wrong shape. Neither crate has
  a `cfg(windows)` line (grep at PR0); `taigi-windows-update` (Authenticode, toast) and
  `taigi-windows-platform` (Win32) stay where they are. A third workspace is required
  rather than optional: a path dependency outside its consumer's workspace must find its
  own root for `version.workspace` / `[lints] workspace = true` to resolve. `desktop/`
  carries the desktop train's version alongside `windows/Cargo.toml`
  (`tools/release_notes.py` `set-versions` / `check-versions`), its own `Makefile`
  (`test` = native `cargo test`, `fmt`, `lint`), and the generated i18n Rust
  (`tools/i18n/generate.py` `WINDOWS_STRINGS_DIR` → `desktop/crates/taigi-desktop-core/src/strings`).
  The Windows `Makefile` `HOST_TESTABLE` roster drops the two crates (tested in
  `desktop/`); `check-gnu` still compiles them for the gnu target through the graph.
  `.claude/rules/windows-guidelines.md`, `docs/architecture/system-overview.md`,
  `windows/README.md` and the i18n rule's grep line are updated in the same PR; the
  memory hand-off notes the move. **Doc comments inside the moved crates keep their
  macOS `file:line` citations** (still the oracle) and lose the words "Windows input
  method" where they describe the crate itself.
- **L3 Settings window = GTK 4 + libadwaita (`gtk4` 0.11, `libadwaita` 0.9 crates).**
  Native GNOME widgets end to end: `adw::ApplicationWindow` + `adw::NavigationSplitView`
  (sidebar `gtk::ListBox` of `adw::ActionRow`s with the pane icons, content
  `adw::NavigationPage` per pane), rows as `adw::PreferencesPage` / `PreferencesGroup` /
  `ActionRow` / `SwitchRow` / `ComboRow` / `EntryRow`, alerts `adw::AlertDialog`,
  banners `adw::Banner`, transient notices `adw::Toast`, the custom-dictionary table a
  `gtk::ColumnView` over a `gio::ListStore`, CSV import / export through
  `gtk::FileDialog`, the shortcut recorder a `gtk::EventControllerKey` on the focused row
  (GTK delivers key events to the app — no hook, unlike WinUI). Adwaita follows the
  system light / dark (`adw::StyleManager` default) as every GNOME app does; the 外觀
  mode row is **not shown** on Linux — it would restyle only this window, while the
  candidate panel's colours are the daemon's (§ L4). Qt /
  KDE parity is **not in this slice**: one toolkit, and IBus's own panel is GTK. The
  window is host-checkable AND host-runnable: `brew install gtk4 libadwaita` on the Mac
  builds the crate natively, so a pane can be opened on the Mac for a visual smoke before
  any Linux dogfood — the gate WinUI never had. Window: `default_size(760, 560)`, frame
  not persisted (same named divergence as Windows W17).
- **L4 Candidates = the framework's panel (platform-adapted presentation).** On Fcitx5 the
  `CommonCandidateList` in the input panel (classic UI / kimpanel, themed by the user), on IBus
  the daemon's lookup table — both fed from the same `Emit::LookupTable`. IBus
  clients do not expose a caret rectangle to a process that owns no window, Wayland
  forbids self-positioned popups without layer-shell, and every IBus engine (rime, mozc,
  chewing, anthy) lets the panel draw. So: `UpdateLookupTable` with the
  `MAX_DISPLAY_CANDIDATES`-capped list the Windows layouts hold, `labels` = the slot-key
  set the 快捷鍵 pane records (`CandidateSlotKeySet`), `page_size` = that cap, `cursor_pos`
  = the highlighted slot, `orientation` from `candidateLayout` (horizontal →
  `HORIZONTAL`; vertical and expandable → `VERTICAL` — the expandable grid has no panel
  equivalent). The preedit is `UpdatePreeditText` with one `UNDERLINE_SINGLE` attribute
  over the whole text, `cursor_pos` = the caret the `UpdatePreedit` effect carries, mode
  `IBUS_ENGINE_PREEDIT_COMMIT` (a focus loss commits what is typed, as the Mac's
  `commitComposition` does; Windows leaves the text as the host left it — `session.rs`
  `composition_terminated`). `CandidateClicked` selects the slot, never commits
  (`CandidateItemView.swift:47-48` — identical semantics); `PageUp` / `PageDown` /
  `CursorUp` / `CursorDown` from the panel run the same `CandidateNavigation` intents the
  keys do. 外觀 rows that the panel owns are **not shown** on Linux: 候選字大小, 候選窗大小,
  字型, and the window's 外觀 mode (§ L3). The 字型管理 pane is **not shown** either
  (USER 2026-09-24 「for linux,拿掉字型管理頁面」, after a report that picking a typeface
  there left the panel unchanged): the panel's font is the framework's — Fcitx5
  › 附加元件 › 經典使用者介面 › 字體 (`classicui.conf` `Font=`), IBus › 偏好設定 › 使用自訂字型
  under `ibus-ui-gtk3`, and nothing per input method under GNOME Shell, whose IBus popup
  reads no font setting (GNOME 46 `ibusCandidatePopup.js`; `custom-font` verified to have
  no effect on the Ubuntu 24.04 VM). The bundled typefaces install as system fonts (L7),
  so they are in those pickers, and a user adds a typeface the system way
  (`~/.local/share/fonts`). A stored / launched `fontManagement` pane lands on 一般.
  `候選窗排列` offers only
  橫 / 直 (a stored expandable reads as 直, the key is never rewritten) and
  `候選字顯示方式` stays. The candidate window
  switch (`candidateWindowEnabled`) maps to "no lookup table" — identical semantics.
  `Effect::DeleteBackwardFromDocument` is a no-op as on macOS / Windows (the preedit is
  never in the document). Two more named divergences the panel forces: the §34 literal
  cell, keyless on macOS / Windows (`lead_cell_is_unkeyed`), takes the first slot key —
  the panel labels every position of every page the same way; and a 合用 cell's other
  script (`CandidateCellContent::annotation`) is drawn after the text in the same cell,
  not as a second line. The auto-space swap (§23) needs the client's surrounding text
  (`IBUS_CAP_SURROUNDING_TEXT` → `DeleteSurroundingText`); a client without it gets
  the mark after the space, as typed (logged, named). The Telex guide (Windows draws a card, macOS a panel) is shown
  as a lookup table of guide rows with no labels and a non-visible cursor, taken down by
  the first key; the symbol picker (`toggle_symbol_picker`) is a lookup table of the
  symbols with the slot-key labels and the category name as auxiliary text — both
  platform-adapted presentation of the same core models (`keys::telex_guide_rows`,
  `keys::symbol_picker`, `symbols::SymbolTable`). The mode flash (`flash_mode_label`) has
  no HUD: the mode is published through the engine's panel property symbol / label
  instead (§ L6) — named divergence.
- **L5 Keys.** `KeyEventSnapshot` is built in `taigi-linux-platform::key_translation`
  from `(keyval, keycode, state)`, the keycode in X terms (evdev + 8): Fcitx5 hands the X
  keycode (`Key::code()`), IBus the evdev code (ibus `client/gtk2/ibusimcontext.c` sends
  `keycode - 8`), converted at the wire by `RawKeyEvent::from_ibus`. `characters` = the
  keysym's Unicode scalar
  (`xkeysym::Keysym::key_char`), `charactersIgnoringModifiers` = the same keysym read as
  if Control were up (IBus hands the keysym the layout produced, so a Ctrl chord's
  keysym is already the letter), `shift` / `ctrl` / `alt` / `win(super)` from the mask
  bits above, the named special keys from the `XK_` constants (`Return 0xff0d`,
  `KP_Enter 0xff8d`, `BackSpace 0xff08`, `Tab 0xff09`, `Escape 0xff1b`, arrows
  `0xff51-0xff54`, `Page_Up/Down 0xff55/0xff56`, `Home/End 0xff50/0xff57`, `Delete
  0xffff`, `space 0x20`). Key releases (`IBUS_RELEASE_MASK`) and bare modifier presses
  answer `false` untouched. Auto-repeat is invisible on the wire, so a held toggle chord
  is latched by the engine until any other key or release arrives — what
  `release_toggle_chord_on_other_key` does on Windows. The 7-tier intent table
  (`taigi-desktop-core::keys::ComposingKeyIntent`) is used unchanged. **Global chords**:
  `openLastSettingsPane` Ctrl+Alt+S, `toggleRomanization` Ctrl+Alt+C, `toggleTranslateSwapped`
  bare `` ` `` — the Windows roster verbatim (macOS ⌃⌘ under ⌘→Ctrl / ⌃→Alt; USER
  2026-08-31 「快捷鍵邏輯必須與 macOS 一致」), matched in the key path while this engine is
  the active one — IBus has no preserved-key registry, so there the chords live exactly as
  long as the Windows fallback path's do; the Fcitx5 addon matches them in `keyEvent` the
  same way (one classifier, two shells), and may later also register them as Fcitx5 hotkeys. **No Shift-tap 中/英 mode** (Windows W5b): IBus
  switches engines (Super+Space) the way macOS switches input sources, which is why the
  Mac has no mode of its own either; every key in English is another engine's.
- **L6 Panel menu = status-area actions (Fcitx5) / engine properties (IBus).** On Fcitx5:
  `SimpleAction`s registered with `userInterfaceManager()` and added to the input context's
  `statusArea()` under `StatusGroup::InputMethod` on `activate` (`fcitx5-rime` `refreshStatusArea`),
  the mode symbol (`chrome::mode_symbol`, what the tray / kimpanel text and the compact
  notice read) through `subModeLabelImpl`, the full mode label through `subMode`. On IBus: `RegisterProperties` on `Enable` and
  on every `FocusIn` with a `PROP_TYPE_MENU` root whose `symbol` is the mode label (the
  panel indicator text — IBus shows an engine's symbol in the top bar) and whose
  sub-properties mirror the Mac's input-source menu row for row
  (`InputSourceMenuRenderer.swift`, `TaigiInputController.swift:372-`): the two switch rows
  under their 快捷鍵-pane names with the recorded chord in the tooltip, a separator, 台語齒盤設定
  (`PropertyActivate` → spawn the settings window on the last pane), a separator, 關於
  (settings window on the 關於 pane) — the shared list's 檢查更新 row is skipped (§ L10).
  Titles are resolved from
  `taigi-desktop-core::strings` in the display language each time the rows are built
  (PR5, `chrome::menu_items`; one list, both shells — since 2026-09-25 the rows themselves
  are `taigi_desktop_core::keys::MENU`, shared with Windows and held equal to the Mac's by test). **Mode label** (PR5,
  `chrome::mode_label`) = `<romanization> · <candidate display mode>` (`台羅 · 漢字優先`),
  the two states the chords switch; a switch emits `Emit::ModeLabel` (the shell re-reads it)
  and `Emit::AnnounceMode` — Fcitx5 `Instance::showInputMethodInformation`, the
  framework's own timed "input method + sub-mode" notice, stands in for the HUD flash;
  IBus has no equivalent and only the symbol changes (named divergence). GNOME Shell reads
  an IBus engine's indicator text from the property whose key is `InputMode`, and only a
  `symbol` of one or two characters (`js/ui/status/keyboard.js`, GNOME 46 — Codex review
  2026-09-23): the menu root's key is therefore `InputMode`, its `symbol`
  `chrome::mode_symbol` (`台羅` / `白話`), its `label` the full `mode_label`. The same
  panel fills an empty label list with `1…9, 0` (`ibusCandidatePopup.js`), so the IBus shell
  sends one space per position for the label-less Telex guide.
- **L7 Data locations (XDG).** `settings.json` under `$XDG_CONFIG_HOME/taigikeyboard/`
  (`~/.config/taigikeyboard/`), the four databases under `$XDG_DATA_HOME/taigikeyboard/`
  (`~/.local/share/taigikeyboard/`); named divergence from Windows' single
  `%APPDATA%\TaigiKeyboard\` — the XDG base-directory spec separates configuration from
  data, and `settings.json` IS what a user would copy between machines. Atomic replace =
  write temp + `rename` (POSIX rename is atomic; the Windows retry loop is not needed and
  not compiled — `taigi-desktop-storage::settings_file` takes the retry policy from the
  caller). Dictionary artefacts are read from `${prefix}/share/taigikeyboard/dictionaries`
  (`dictionary.fst`, `dictionary.bin`, `association.bin`, `syllables.fst` — the
  repo-root `dictionaries/` copied at install), `${prefix}` baked at build time from
  `TAIGIKEYBOARD_PREFIX` (default `/usr`); `TAIGIKEYBOARD_DATA_DIR` at runtime overrides
  it for a development tree (`make -C linux run-engine`). The four bundled typefaces go to
  `${prefix}/share/fonts/{truetype,opentype}/taigikeyboard/` as system-wide fallbacks
  (`𧉟` U+2725F has no glyph in Noto CJK; all four bundled faces have it) and as
  choices in the framework's font picker (L4); the panel draws in the font picked there.
- **L8 Settings launcher.** The engine spawns `${prefix}/bin/taigikeyboard-settings` with
  the shared `taigi-desktop-core::settings::launch` contract (`--pane <raw>`; `--check-now`
  / `--check-updates` / `--prewarm` are Windows-only and rejected on Linux with a readable
  error). The component XML's `<setup>` points at the same binary so ibus-setup's
  "Preferences" button opens it. Single instance through `gio::Application` with an
  application id (`tw.taigikeyboard.Settings`): a second launch with a different `--pane`
  activates the running window on that pane (the Windows mutex contract, native here).
- **L9 Settings live reload** = Windows W10 unchanged: `settings.json` carries the
  monotonically increasing `revision`, the engine uses mtime / size as the cheap change
  detector on `FocusIn` and on the first key of each composition, then adopts the new
  revision; a parse failure keeps last-known-good; which keys may change mid-composition
  follows macOS invariant §11 item by item. The settings window re-reads on a 1 s
  `glib::timeout_add_local` tick while open (the Windows tick), so the recorder's
  conflict pass and the engine's chords agree.
- **L10 Update check: none — the package manager's job (restored 2026-09-25).** An input
  method is a system package (Fcitx5 and IBus load it from system paths), and a Linux
  packager's review of the 2026-09-24 reversal put it plainly: an input method has no
  business checking its own updates; an update nag works against the distribution that
  packages it. USER 2026-09-25: 「for desktop,只有macos,windows需要檢查更新功能,linux不需要，可以整個拿掉」.
  The manual and automatic checks (#176, #178, never released) are removed whole: no
  檢查更新 row in the panel menu (`chrome::menu_id` skips the shared
  `MenuCommand::CheckForUpdates`), no engine trigger, no `--check-now` / `--check-updates`
  (refused, L8), no D-Bus service file, no `appcast/linux.json`, and the settings binary
  links no `taigi-desktop-update` — no TLS stack in the package. The 一般 pane keeps the
  running version and a 去下載 link to taigikeyboard.tw. No build flag either: there is
  nothing to switch off. Spawned children stay reaped on a thread (`launcher::spawn_detached`).
- **L11 Packaging + release.** `make -C linux install PREFIX=/usr DESTDIR=` installs the two
  binaries, the component XML (rendered with the prefix), the dictionaries, a
  `tw.taigikeyboard.Settings.desktop` entry + icon, and prints the `ibus restart`
  reminder; `uninstall` reverses it and keeps `$XDG_*/taigikeyboard`. A `.deb` is packed by
  `dpkg-deb` over that same install layout (`make -C linux deb`: `make install DESTDIR`,
  `packaging/control.in`, `Depends` from `dpkg-shlibdeps` + `fcitx5 | ibus`; both shells in
  one package, as `fcitx5-chewing` + `ibus-chewing` come from one source; revised 2026-09-23
  from the `cargo-deb` plan — a second asset list would drift from `make install`) — on the
  GitHub-hosted Ubuntu runner (`.github/workflows/linux-build.yml`, mirror of
  `windows-build.yml`: built on every PR; a `main` `workflow_dispatch` from
  `scripts/stage-desktop.sh` attaches the `.deb` + SHA-256 to the `desktop-<version>` DRAFT,
  never over an existing asset and never on a publish — `linux-release.md`). No signing
  (no Linux-side equivalent of Authenticode / notarization is expected of a `.deb`
  downloaded from a project page; apt-repository signing is outside this slice). Version
  source of truth stays `windows/Cargo.toml`; `make version-desktop x.y.z` moves
  `desktop/Cargo.toml` and `linux/Cargo.toml` with it. **RPM / Flatpak / AUR are not in
  this slice** (Flatpak cannot host an IBus engine at all).
- **L12 Verification without a Linux machine.** Per PR, `make linux-check` from the repo
  root: `cargo test` for `desktop/` (native, the moved crates' existing tests),
  `cargo clippy --workspace --all-targets -- -D warnings` for `linux/` natively on the Mac
  (`zbus` and gtk4-rs both build on macOS; libadwaita from Homebrew), a REAL cross build
  of the engine binary for `x86_64-unknown-linux-gnu` through `cargo zigbuild` (zig ships
  the C toolchain the bundled SQLite's build script needs — plain `cargo check` stops
  there; `brew install zig cargo-zigbuild`), `cargo fmt -- --check`, and the i18n check. CI (`linux-build.yml`, nightly
  and on dispatch since 2026-09-25) adds what the Mac cannot: a real
  `x86_64-unknown-linux-gnu` build of both binaries with the distro's GTK, the **Fcitx5 addon
  built with CMake against `fcitx5-modules-dev`** (the only place it compiles before the VM),
  `cargo test` of the whole `linux/` workspace, the `.deb` (`make deb`, contents asserted), and an **IBus daemon smoke**: `dbus-run-session`
  → `ibus-daemon --daemonize --panel disable` with the component XML installed into a
  temporary `IBUS_COMPONENT_PATH`, then `ibus list-engine | grep taigikeyboard` and
  `ibus engine taigikeyboard` — proof that the daemon can spawn the engine and complete
  `CreateEngine`, which is the wire's first byte-level contract. The settings window is
  additionally mounted headlessly: each pane is built against an offscreen
  `gtk::Window` in a test (`GDK_BACKEND=broadway` / `xvfb-run` on CI), the equivalent of
  Windows' `pane_planning` net. What no check catches — actual preedit rendering in a
  GTK / Qt / Chromium client, Wayland text-input v3 behaviour, the panel's label
  handling — is the dogfood run-book's.
- **L13 Threading + ownership.** `zbus` runs the object server on its own executor; every
  `ProcessKeyEvent` arrives serially per connection, so engine state lives in one
  `Mutex<Runtime>` (settings provider, `ComposingSessionCoordinator`, per-engine state)
  and no call ever awaits while holding it. Each `CreateEngine` makes one engine object
  = one `ContextToken`; the coordinator's handover rule (Windows W3: a key from context B
  while A owns the engine commits A under A's own path) is reused with `CommitText` on
  A's object. `Destroy` / connection loss releases the token. The engine initialises
  the lexicon lazily on the first consumed key (rakukan `factory.rs:413-419`, Windows
  `prepare_for_first_key`). Panics: every D-Bus method body runs under `catch_unwind`
  and answers `false` / `()` — a panic must not take the engine process down while a
  user is mid-word (the daemon would respawn it and lose the composition).

## Named divergences (summary table)

| Behaviour | macOS | Windows | Linux | Class |
|---|---|---|---|---|
| Candidate window | own `NSPanel`, 3 layouts | own D2D popup, 3 layouts | daemon lookup table, orientation from layout | platform-adapted presentation |
| 外觀 rows 候選字大小 / 候選窗大小 / 字型 | yes | yes | hidden (panel-owned) | unsupported host capability |
| 字型管理 pane | selects the candidate typeface | selects the candidate typeface | hidden (panel-owned; font set in Fcitx5 / IBus) | unsupported host capability |
| Focus loss mid-composition | client commits | text stays as host left it | daemon commits (`PREEDIT_COMMIT`) | platform-adapted |
| 中/英 Shift tap | none (OS switches sources) | yes | none (IBus switches engines) | identical to macOS |
| Global chords | Carbon hotkeys, session-scoped | preserved keys + fallback | matched in `ProcessKeyEvent` while active | identical semantics |
| Mode flash HUD | yes | yes | panel property symbol / label | platform-adapted |
| Menu | input-source menu | tray button menu | panel property menu | platform-adapted |
| Update check | Sparkle-style in-app | scheduled task + in-app | none; version + download link (§ L10) | named divergence: the package manager updates a Linux input method |
| Data dir | `~/Library/Application Support/<bundle>` | `%APPDATA%\TaigiKeyboard` | XDG config + data split | platform-adapted |
| Settings frame persisted | yes | no | no | named divergence |
| 教典 off: its eleven 腔口 rows | shown, greyed | shown, greyed | shown, greyed (USER 2026-09-23: mirror the other desktops; the expander of PR7 reverted) | identical semantics |
| Sidebar icons | SF Symbols | Fluent | Adwaita symbolic (`preferences-system`, `applications-graphics`, `input-keyboard`, `emblem-documents`, `x-office-address-book`) | identical semantics |
| 自訂詞庫 table | two columns 羅馬字 / 漢字, double-click edits | two columns, ✎ edits | two columns, double-click or ✎ edits | identical semantics |
| Release flow | `make desktop-release` stages the `.pkg` | the `.exe` on the same draft | the `.deb` on the same draft (`linux-build.yml` `attach`); announce writes `_data/linux_release.json` (download buttons only, no appcast, § L10) | identical semantics |

## Phase / PR table

USER sizing preference (2026-08-17): ~600–1000 LOC per PR, fewer and larger. Every coding
PR runs `make linux-check` (§ L12) and, from PR3, the CI job. Codex sandwich: skipped on
PR0 (quota, 2026-09-22); each later PR records its own verdict here.

| PR | Phase | Scope | Status |
|---|---|---|---|
| PR0 | Admin | this roadmap + `.claude/rules/linux-guidelines.md` + docs index + memory topic | this PR |
| PR1 | Proto | `PLATFORM_LINUX = 5` in `envelope.proto` + committed platform stubs regenerated (mechanical, its own PR as W12) | pending |
| PR2 | Crate move | `desktop/` workspace: `taigi-desktop-core` + `taigi-desktop-storage` moved + renamed; `windows/` re-pointed; `windows/Makefile` rosters; `tools/i18n/generate.py` output path + `linux` in `VALID_PLATFORMS` with every `windows`-scoped key also scoped `linux`; `tools/release_notes.py` version files; root `Makefile` `desktop-check` / `linux-check`; docs + rules references; `make windows-check` green | pending |
| PR3 | Engine I — wire | `linux/` workspace + toolchain; `taigi-linux-platform` (XDG paths, prefix, key translation, launcher, open URL; host stubs none needed); `taigikeyboard-ibus`: bus address + connection, factory, engine object with the full key path (snapshot → intent → manager inside the runtime lock → preedit / commit / lookup table), focus + reset + destroy lifecycle, wire types with signature tests; component XML template; `linux/Makefile`; `linux-build.yml` with the daemon smoke | MERGED #143 `28b3e4f6` 2026-09-23 — Codex skipped (quota) |
| PR4 | Fcitx5 shell | `taigi-linux-core` extracted from the IBus crate (runtime, session, executor `Emit`, selection; IBus crate rebased on it); `taigi-linux-ffi` staticlib + `taigikeyboard.h` C ABI with a header-compiles test; `linux/fcitx5/` CMake addon (`InputMethodEngineV3`, client preedit, `CommonCandidateList`, commit, delete-surrounding, reset / deactivate, addon + inputmethod `.conf`); `linux/Makefile` `build-fcitx5` / install into `${libdir}/fcitx5`; CI builds the addon | MERGED #144 `68dfc6b3` 2026-09-23 — Codex skipped (quota); CI facts: Ubuntu 24.04 ships fcitx5 **5.1.7** — no `add_fcitx5_addon` (5.1.12+) and no `FCITX_ADDON_FACTORY_V2`, so the addon uses `add_library(MODULE)` + `FCITX_ADDON_FACTORY`; the `.conf` files install under `${CMAKE_INSTALL_DATADIR}/fcitx5` (`FCITX_INSTALL_PKGDATADIR` is always `/usr/share/fcitx5`); `make -C linux check-cpp` syntax-checks the C++ on the Mac over a `references/fcitx5` 5.1.7 clone |
| PR5 | Chrome, both shells | status-area actions (Fcitx5) / properties menu (IBus) per § L6, settings live reload (§ L9), global chords + toggle latch, symbol picker + Telex guide as candidate lists, mode label (`subModeLabelImpl` / property symbol), `run-engine` dev target | MERGED #147 `076041db` 2026-09-23 (#146 auto-closed with its base branch) — `chrome.rs` in `taigi-linux-core`: chords + latch, Telex guide and symbol picker as lookup tables, `menu_items` / `mode_label` / `mode_symbol` / `Emit::ModeChanged` + `AnnounceMode`; Fcitx5 status-area `SimpleAction`s + `subMode` / `subModeLabelImpl` + `showInputMethodInformation`, panel paging routed through the core; IBus `RegisterProperties` root `InputMode` + `PropertyActivate`; live reload was already `LiveSettings::current()` (L9). Codex post-impl FIX → applied |
| PR6 | Settings I | `taigikeyboard-settings`: `adw` shell (sidebar, pane routing, `--pane`, single instance, display language, live tick), 一般, 外觀 (Linux row set), 關於 | this PR (branch `feat/linux-settings-shell`) — crate `taigikeyboard-settings` (lib + bin): `adw::Application` `tw.taigikeyboard.Settings` with `HANDLES_COMMAND_LINE` (second launch re-activates on `--pane`), `NavigationSplitView` sidebar + `gtk::Stack` of `adw::PreferencesPage`s, write-failure / read-only `adw::Banner`, 1 s `glib::timeout_add_local` live tick (a display-language change rebuilds the pages), `StyleManager` colour scheme from 外觀; 一般 (Linux row set + version / 去下載 row, no update check) / 外觀 (mode · 候選窗排列 · 候選詞顯示) / 關於; Windows-only flags refused by name; `tests/panes.rs` (`harness = false`; mounts the whole window, then: every built pane in the stack, a switch row writes its key, an outside write is adopted on the tick without a revision bump, a language picked in the window rebuilds the sidebar, an unbuilt `--pane` lands on 一般, the read-only window writes nothing, reset keeps the language; skips without a display unless `TAIGI_REQUIRE_DISPLAY` — set on CI under xvfb); `make -C linux run-settings` opens the window on the Mac. Codex post-impl **FIX → applied** (local write follows the same rebuild path as an outside one; unbuilt panes route to 一般 and `--pane` is remembered; read-only = in-memory defaults, never a temp file; the content `NavigationPage` title is what the header bar draws (libadwaita 1.5); refused flags print to the caller's stderr via `printerr_literal` (gio `v2_80`)) |
| PR7 | Settings II | 快捷鍵 (recorder over `EventControllerKey`, both registries, conflicts, slot-key-set picker), 詞庫來源 (教典 subcollections in an `adw::ExpanderRow`) | MERGED #149 2026-09-23 — 快速齒 (recorder over a capture-phase `EventControllerKey`, keycode latch, recording ends on pane switch / other write / focus loss) + 詞庫來源 (教典 `ExpanderRow`); Codex FIX applied |
| PR8 | Settings III | 自訂詞庫 (`ColumnView` table, paging, CRUD dialog, CSV via `FileDialog`, delete all, clear learning — background work on a `gio` task with the 400 ms busy card), unlisted 辭典搜尋 + external lookup URLs; headless pane-mount test | MERGED #150 2026-09-23 — 自訂詞庫 + 辭典搜尋; Codex FIX applied (render snapshot, window `JobSlot`, `changed` generations, banner for the data dir, lexicon retry, weak dialog, `use_markup(false)`) |
| PR9 | Packaging + release | `make -C linux install / uninstall` (both shells), `.desktop` + icon (`tools/desktop/make-app-icon.swift` PNG set), `cargo-deb` metadata, CI `.deb` artifact on release publish, `scripts/stage-desktop.sh` dispatch, `docs/architecture/linux-release.md`, `desktop-release.md` + `system-overview.md` + README rows, `S74` dogfood item (VM: KDE Plasma + Fcitx5 first, then GNOME + IBus) | MERGED #151 2026-09-23 — `make -C linux deb` (dpkg-deb over the install layout, `Depends` from dpkg-shlibdeps: `fcitx5 | ibus, libadwaita-1-0 (>= 1.5~beta), libc6 (>= 2.39), libfcitx5core7 (>= 5.1.7), libfcitx5utils2 (>= 5.1.7), libgcc-s1, libglib2.0-0t64 (>= 2.79.0), libgtk-4-1 (>= 4.9.3), libstdc++6 (>= 13.1)`), `.desktop` + hicolor icons, `linux-build.yml` build/attach split, `stage-desktop.sh` both hosted runs, `linux-release.md`, S74; Codex FIX applied (the addon file is `libtaigikeyboard.so` — the `Library=export:` name — never `--clobber`, `source_sha` guard, read-only build job) |

Dependencies: PR1 → PR2 → PR3 → PR4 → PR5; PR2 → PR6 → PR7 → PR8; PR9 last. PR4/PR5 and
PR6–PR8 are parallelisable after PR3.

## Shared-surface coordination register

| Change | PR | Additive? | ios/android/macos files touched? |
|---|---|---|---|
| `PLATFORM_LINUX = 5` in `envelope.proto` + regen | PR1 | yes | **yes — generated `.pb.swift` / `.java` only, semantically inert** |
| `windows/crates/{core,storage}` → `desktop/crates/…`, package rename | PR2 | move | `windows/**` (manifests, `use` paths, Makefile, README, rules, docs) |
| `linux` joins `VALID_PLATFORMS`; every `windows`-scoped key gains `linux` | PR2 | yes | `i18n/*.json` scope arrays only; no value changes |
| `tools/release_notes.py` version files += `desktop/Cargo.toml`, `linux/Cargo.toml` | PR2 | yes | no |
| Root `Makefile` `desktop-check` / `linux-check` / `version-desktop` | PR2 | yes | no |
| `scripts/stage-desktop.sh` dispatches `linux-build.yml`; announce workflow Linux asset | PR8 | yes | separate repo for the website half |

## 最佳實踐對齊 (references)

| 主流做法 | 來源 | 本 plan 對應 |
|---|---|---|
| Engine as a separate process the daemon spawns from a component XML; panel draws preedit + candidates | ibus `src/ibusfactory.c`, `src/ibusengine.c`; rime `ibus-rime`, mozc `unix/ibus/` | L1, L4 |
| Serialisable wire tuples `(sa{sv}…)` in a fixed field order kept for compatibility | ibus `src/ibusserializable.c:277`, `src/ibusenginedesc.c` ("The serialized order should be kept") | L1 |
| Shell-independent core tested natively, thin platform shells | `windows-roadmap.md` W1 (Codex CONFIRM), `references/ChiaKey` `ChiaKeyCore` facade | L2, L13 |
| Blind authoring gated by cross-target checks + hosted runner | `windows-roadmap.md` W13 + `project_windows_hosted_build` | L12 |
| XDG base directories for config vs data | freedesktop basedir spec; `ibus` itself (`g_get_user_config_dir()` for its bus file, `src/ibusshare.c:195`) | L7 |
| No self-update check in an input method; the distribution updates the package | distro IBus / Fcitx5 engines (`ibus-rime`, `ibus-mozc`, `fcitx5-chewing`) never check; Linux packager review 2026-09-25 | L10 |
| libadwaita preference widgets for an IME's settings | GNOME Settings, `ibus-setup` (GTK), `ibus-anthy` setup dialog | L3 |
| Lazy engine init on first consumed key | rakukan `factory.rs:413-419`; Windows `Runtime::prepare_for_first_key` | L13 |

**Deliberately not adopted**: an own GTK candidate popup (unpositionable under Wayland;
no IBus engine does it); libibus FFI bindings (C toolchain on the host, GObject
ownership across FFI, and nothing the D-Bus surface lacks); (2026-09-22 only) Fcitx5 addon — reversed 2026-09-23, see L1; Qt settings window (second toolkit; the IBus panel is GTK);
any update check, manual or automatic, and a build flag to switch one off (§ L10: nothing to switch); Flatpak (cannot host an IBus engine); a `linux/` crate
that depends on `../windows/crates/…` (wrong shape, § L2). **YAGNI**: surrounding-text
capability (nothing in the key table reads the document); handwriting methods answer
`()`; `ForwardKeyEvent` is unused (a key the engine does not consume is answered `false`
and the client processes it itself).

## Dogfood run-book (first real desktop)

Ordered acceptance for the first machine (Ubuntu / Fedora GNOME on Wayland, then an X11
session, then a KDE session with IBus): (1) `make -C linux install` → `ibus restart` →
the engine appears in Settings › Keyboard › Input Sources under 台語 (nan) with the 台
symbol; (2) S1 POJ diacritics + S2 TPS-off + S3 candidate paging on the panel in gedit /
GTK4 text view, Firefox, a Qt app, a terminal; (3) slot-key labels match the 快捷鍵 pane;
(4) focus loss commits the preedit; (5) Ctrl+Alt+S opens the settings window on the last
pane, second launch re-activates it; (6) Ctrl+Alt+C / bare `` ` `` switch and the panel
symbol follows; (7) every settings row round-trips through `settings.json` and the engine
picks it up on the next composition; (8) 自訂詞庫 CRUD + CSV; (9) `.deb` install /
remove keeps `~/.config/taigikeyboard` + `~/.local/share/taigikeyboard`; (10) high
contrast + large text; (11) a GNOME session with the engine on an external keyboard with
a non-US layout (keysym vs keycode). Each `Sn` line lands in `dogfood-checklist.md` with
its PR.
