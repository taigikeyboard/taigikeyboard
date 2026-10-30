# Linux Desktop IME — Roadmap

> **Type**: Reference (design record)
> **Keywords**: `linux`, `IBus`, `Fcitx5`, `D-Bus`, `zbus`, `GTK4`, `libadwaita`, `desktop`, `engine reuse`, `macOS parity`, `Windows parity`
> **Status**: implemented; first-machine dogfood S74 passed (`dogfood-checklist.md`). Fcitx5 is the primary frontend, the IBus engine the second (§ L1).
> **Siblings**: `macos-roadmap.md`, `windows-roadmap.md` (the crates this platform reuses). Release flow: `linux-release.md`.
> **Behaviour oracle**: desktop-core and its tests, for all three desktops (`macos-desktop-core-roadmap.md` D8). "macOS is the oracle" wording and macOS `file:line` cites below predate that.

---

## Goal

Linux is the third desktop platform beside macOS and Windows, designed consistent with both, with a settings window built from native UI components. Three consequences shape every decision below:

1. **One shared desktop core.** Every observable desktop behaviour (key table, modes,
   auto-space, full-width punctuation, settings keys and defaults, storage schemas, CSV,
   shortcuts) lives in `desktop/crates/taigi-desktop-core` + `-storage`. Linux reuses the
   Rust verbatim (§ L2); only the shell is new. Deltas are named with the Windows five-way
   classification (**identical semantics** · **platform-adapted presentation** ·
   **unsupported host capability** · **intentionally not in this slice** · **unverified
   until Linux dogfood**), per `docs/contributing/cross-platform-alignment.md` §3.
2. **Native widgets for the settings window.** On Linux "native" is the desktop's
   toolkit: GTK 4 + libadwaita (§ L3), the toolkit GNOME Settings, `ibus-setup` and the
   GNOME IME preference dialogs are built with.
3. **Host-checkable from a Mac.** `make linux-check` (§ L12) and the Ubuntu CI job gate
   every change; what they cannot catch is a device dogfood item.

**Hard constraints**: release / tag / package actions stay maintainer-gated. Shared-surface
changes (proto, i18n, release tooling) name every platform they touch.

## Architecture

Everything below the shells is shared. The Fcitx5 shell
is `linux/fcitx5/libtaigikeyboard.so` (C++ `InputMethodEngineV3` over the `taigi-linux-ffi` C ABI
over `taigi-linux-core`); the IBus shell is `ibus-engine-taigikeyboard` (zbus over the same
`taigi-linux-core`). The diagram draws the IBus process; the Fcitx5 addon
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
│  adw::ApplicationWindow + NavigationSplitView sidebar: General / Appearance /            │
│  Shortcuts / Dictionary Sources / Custom Dictionary / Learning Records / About (+ unlisted│
│  Dictionary Search); PreferencesPage / PreferencesGroup / ActionRow /                    │
│  SwitchRow / ComboRow; AlertDialog; Banner; ColumnView table; gtk::FileDialog CSV        │
└─────────────────────────────────────────────────────────────────────────────────────────┘
Runtime data: $XDG_CONFIG_HOME/taigikeyboard/settings.json ·
$XDG_DATA_HOME/taigikeyboard/{user_frequency.db, user_association.db, custom_dictionary.db,
learned_phrases.db}. Install: ${prefix}/libexec/ibus-engine-taigikeyboard,
${prefix}/bin/taigikeyboard-settings, ${prefix}/share/ibus/component/taigikeyboard.xml,
${prefix}/share/taigikeyboard/dictionaries/*, ${prefix}/share/applications/…desktop, icons.
```

## Design decisions (L1–L13, grounded in code and in the ibus source)

Citations: `ibus src/*.c` is github.com/ibus/ibus `main` as fetched 2026-09-22 (the
introspection XML and the `serialize` functions were read, not remembered).

- **L1 Frameworks = Fcitx5 (primary) + IBus (second), one Rust core, two thin shells.**
  IBus came first only because, without a Linux machine, a pure-D-Bus engine was the one
  thing the Mac could compile and check. Fcitx5 is where Taiwanese / CJK Linux users are (fcitx5-chewing,
  fcitx5-rime, KDE's default), has the most complete Wayland story, draws its own themed
  candidate panel, and offers real global hotkeys. Mainstream engines ship both (`ibus-rime`
  + `fcitx5-rime`, `ibus-mozc` + `fcitx5-mozc`, `ibus-chewing` + `fcitx5-chewing`), which
  is the shape adopted:
  - **`taigi-linux-core`** (Rust lib, pure): the framework-independent half of the original IBus crate — `Runtime` (settings, stores, lexicon, coordinator), the key path
    (`session.rs`: snapshot → intent → manager → `Emit`s), the `Emit` effect model
    (preedit / commit / delete-surrounding / lookup table), `LookupSelection`. Both shells
    replay the same `Emit` list; nothing composes in a shell.
  - **`taigi-linux-ffi`** (Rust `staticlib`, the ONE Linux crate allowed `unsafe`, `// SAFETY:`
    per call as `rust-ffi-safety.md` §3): a C ABI over `taigi-linux-core` — opaque runtime /
    engine handles, `taigi_engine_key(keysym, keycode, states) → reply`, accessor functions
    over the reply's emits (kind, text, caret, table rows / labels / cursor / orientation), a
    hand-written `taigikeyboard.h` pinned by a test that compiles it. `catch_unwind` at every
    entry (`docs/contributing/rust-ffi-safety.md` §1.2), never a panic across the boundary.
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
  - **`taigikeyboard-ibus`** sits on `taigi-linux-core`: the D-Bus
    wire (`bus`, `wire`, `factory`, `engine`) is all that remains in it. GNOME users get it
    from the same package.
  The IBus wire facts below stay authoritative for that shell. What the two shells must
  agree on is the `Emit` contract, so every divergence between them is a shell bug, not a
  design choice.
  **The IBus shell**: engine in pure Rust over D-Bus (`zbus` 5) — no `libibus`,
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
    `taigikeyboard`, `<language>nan</language>`, `<layout>default</layout>` (not
    `us`: GNOME applies an engine's layout on activation, so `us` forced QWERTY on
    Dvorak / Colemak users; `default` keeps the active layout, as Fcitx5 keeps the group
    layout — switching away from a non-Latin layout, or TPS on a non-US layout, is the
    user's; `ibus engine taigikeyboard` now exits 1 after switching, since it has no
    layout to hand setxkbmap — CI reads the engine back instead), `<symbol>台</symbol>`,
    `<setup>${prefix}/bin/taigikeyboard-settings</setup>`, `<rank>0</rank>`). `ibus-daemon`
    reads the directory at start (or after `ibus write-cache`), spawns the exec on demand,
    and the daemon — not the engine — owns activation, so no `RegisterComponent` call.
  **Risk**: the exact `a{sv}` attachment shape
  and the `v`-wrapping of nested serialisables are the two places a byte-level mistake
  would show only on a real daemon; both are pinned by unit tests against the signature
  strings `(sa{sv}sv)` etc. and by the CI smoke (§ L12).
- **L2 Crate sharing = a `desktop/` Cargo workspace.** `windows/crates/taigi-windows-core`
  → `desktop/crates/taigi-desktop-core`, `taigi-windows-storage` →
  `taigi-desktop-storage`, package + module names renamed mechanically, consumed by both
  `windows/` and `linux/` by path — the same relationship both have to `engine/`. Core
  Principle #5 (direction-first): a Linux workspace that depends on
  `../windows/crates/…` would be the smaller diff and the wrong shape. Neither crate has
  a `cfg(windows)` line; `taigi-windows-update` (Authenticode, toast) and
  `taigi-windows-platform` (Win32) stay where they are. A third workspace is required
  rather than optional: a path dependency outside its consumer's workspace must find its
  own root for `version.workspace` / `[lints] workspace = true` to resolve. `desktop/`
  carries the desktop train's version alongside `windows/Cargo.toml`
  (`tools/release_notes.py` `set-versions` / `check-versions`), its own `Makefile`
  (`test` = native `cargo test`, `fmt`, `lint`), and the generated i18n Rust
  (`tools/i18n/generate.py` `WINDOWS_STRINGS_DIR` → `desktop/crates/taigi-desktop-core/src/strings`).
  The Windows `Makefile` `HOST_TESTABLE` roster drops the two crates (tested in
  `desktop/`); `check-gnu` still compiles them for the gnu target through the graph.
  **Doc comments inside the moved crates keep their macOS `file:line` citations** and
  do not call the crate a "Windows input method".
- **L3 Settings window = GTK 4 + libadwaita (`gtk4` 0.11, `libadwaita` 0.9 crates).**
  Native GNOME widgets end to end: `adw::ApplicationWindow` + `adw::NavigationSplitView`
  (sidebar `gtk::ListBox` of `adw::ActionRow`s with the pane icons, content
  `adw::NavigationPage` per pane), rows as `adw::PreferencesPage` / `PreferencesGroup` /
  `ActionRow` / `SwitchRow` / `ComboRow` / `EntryRow`, alerts `adw::AlertDialog`,
  banners `adw::Banner`, transient notices `adw::Toast`, the custom-dictionary table a
  `gtk::ColumnView` over a `gio::ListStore`, CSV import / export through
  `gtk::FileDialog`, the shortcut recorder a `gtk::EventControllerKey` on the focused row
  (GTK delivers key events to the app — no hook, unlike WinUI). Adwaita follows the
  system light / dark (`adw::StyleManager` default) as every GNOME app does; the Appearance
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
  set the Shortcuts pane records (`CandidateSlotKeySet`), `page_size` = that cap, `cursor_pos`
  = the highlighted slot, `orientation` from `candidateLayout` (horizontal →
  `HORIZONTAL`; vertical and expandable → `VERTICAL` — the expandable grid has no panel
  equivalent). The preedit is `UpdatePreeditText` with one `UNDERLINE_SINGLE` attribute
  over the whole text, `cursor_pos` = the caret the `UpdatePreedit` effect carries, mode
  `IBUS_ENGINE_PREEDIT_COMMIT` (a focus loss commits what is typed, as the Mac's
  `commitComposition` does; Windows leaves the text as the host left it — `session.rs`
  `composition_terminated`). `CandidateClicked` selects the slot, never commits
  (`CandidateItemView.swift:47-48` — identical semantics); `PageUp` / `PageDown` /
  `CursorUp` / `CursorDown` from the panel run the same `CandidateNavigation` intents the
  keys do. Appearance rows that the panel owns are **not shown** on Linux: Candidate Text
  Size, Candidate Window Size, Font, and the window's Appearance mode (§ L3). The Manage
  Typefaces pane is **not shown** either
  (picking a typeface there left the panel unchanged): the panel's font is the framework's — Fcitx5
  › Addons › Classic User Interface › Font (`classicui.conf` `Font=`), IBus › Preferences ›
  Use custom font under `ibus-ui-gtk3`, and nothing per input method under GNOME Shell, whose IBus popup
  reads no font setting (GNOME 46 `ibusCandidatePopup.js`; `custom-font` verified to have
  no effect on the Ubuntu 24.04 VM). The bundled typefaces install as system fonts (L7),
  so they are in those pickers, and a user adds a typeface the system way
  (`~/.local/share/fonts`). A stored / launched `fontManagement` pane lands on General.
  `Candidate Window Layout` offers only
  Horizontal / Vertical (a stored expandable reads as Vertical, the key is never rewritten) and
  `Candidate Display Mode` stays. The candidate window
  switch (`candidateWindowEnabled`) maps to "no lookup table" — identical semantics.
  The §34 literal cell is keyless as on macOS / Windows (both panels page through the
  core, which sends the labels with every table; IBus gets a space for the blank label so
  GNOME does not number it). One more
  named divergence the panel forces: under the combined
  (mixed) display option a cell's other script (`CandidateCellContent::annotation`) is drawn after the text in the same cell,
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
  bare `` ` `` — the Windows roster verbatim (macOS ⌃⌘ under ⌘→Ctrl / ⌃→Alt — the
  shortcut logic matches macOS), matched in the key path while this engine is
  the active one — IBus has no preserved-key registry, so there the chords live exactly as
  long as the Windows fallback path's do; the Fcitx5 addon matches them in `keyEvent` the
  same way (one classifier, two shells), and may later also register them as Fcitx5 hotkeys. **No Shift-tap Chinese/English mode** (Windows W5b): IBus
  switches engines (Super+Space) the way macOS switches input sources, which is why the
  Mac has no mode of its own either; every key in English is another engine's.
- **L6 Panel menu = status-area actions (Fcitx5) / engine properties (IBus).** On Fcitx5:
  `SimpleAction`s registered with `userInterfaceManager()` and added to the input context's
  `statusArea()` under `StatusGroup::InputMethod` on `activate` (`fcitx5-rime` `refreshStatusArea`),
  the mode indicator (`chrome::mode_indicator`, invariants §60) as the tray icon through
  `subModeIconImpl` (a hicolor icon name, as fcitx5-mozc does — the tray draws the icon unless
  Prefer Text Icon is on) and as the tray / kimpanel text and compact notice through
  `subModeLabelImpl`, the full mode label through `subMode`. On IBus: `RegisterProperties` on `Enable` and
  on every `FocusIn` with a `PROP_TYPE_MENU` root whose `symbol` and `icon` are the mode
  indicator (the panel indicator — GNOME Shell shows the symbol in the top bar; the component
  file names the root as `<icon_prop_key>` for the other panels) and whose
  sub-properties mirror the Mac's input-source menu row for row
  (`InputSourceMenuRenderer.swift`, `TaigiInputController.swift:372-`): the two switch rows
  under their Shortcuts-pane names with the recorded chord in the tooltip, a separator, TaigiKeyboard Settings
  (`PropertyActivate` → spawn the settings window on the last pane), a separator, About
  (settings window on the About pane) — the shared list's Check for Updates row is skipped (§ L10).
  Titles are resolved from
  `taigi-desktop-core::strings` in the display language each time the rows are built
  (`chrome::menu_items`; one list, both shells — the rows themselves
  are `taigi_desktop_core::keys::MENU`, shared with Windows and held equal to the Mac's by test). **Mode label**
  (`chrome::mode_label`) = `<romanization> · <candidate display mode>` (`台羅 · 漢羅對應`),
  plus ` · <output script>` under side by side (`台羅 · 漢羅對應 · 漢字`); a switch emits
  `Emit::ModeChanged` (the shell re-reads the label and the indicator; the swap emits it
  too, announced only under side by side)
  and `Emit::AnnounceMode` — Fcitx5 `Instance::showInputMethodInformation`, the
  framework's own timed "input method + sub-mode" notice, stands in for the HUD flash;
  IBus has no equivalent and only the symbol changes (named divergence). GNOME Shell reads
  an IBus engine's indicator text from the property whose key is `InputMode`, and only a
  `symbol` of one or two characters (`js/ui/status/keyboard.js`, GNOME 46): the menu root's key is therefore `InputMode`, its `symbol`
  `ModeIndicator::symbol` (`台` / `Ts` / `白` / `Ch` / `方`), its `label` the full `mode_label`. The same
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
  repo-root `assets/dictionaries/` copied at install), `${prefix}` baked at build time from
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
- **L10 Update check: none — the package manager's job.** An input
  method is a system package (Fcitx5 and IBus load it from system paths); an input method
  has no business checking its own updates, and an update nag works against the
  distribution that packages it. Only macOS and Windows check for updates. There is no
  Check for Updates row in the panel menu (`chrome::menu_id` skips the shared
  `MenuCommand::CheckForUpdates`), no engine trigger, no `--check-now` / `--check-updates`
  (refused, L8), no D-Bus service file, no `appcast/linux.json`, and the settings binary
  links no `taigi-desktop-update` — no TLS stack in the package. The General pane keeps the
  running version and a Download link to taigikeyboard.tw. No build flag either: there is
  nothing to switch off. Spawned children stay reaped on a thread (`launcher::spawn_detached`).
- **L11 Packaging + release.** `make -C linux install PREFIX=/usr DESTDIR=` installs the two
  binaries, the component XML (rendered with the prefix), the dictionaries, a
  `tw.taigikeyboard.Settings.desktop` entry + icon, and prints the `ibus restart`
  reminder; `uninstall` reverses it and keeps `$XDG_*/taigikeyboard`. A `.deb` is packed by
  `dpkg-deb` over that same install layout (`make -C linux deb`: `make install DESTDIR`,
  `packaging/control.in`, `Depends` from `dpkg-shlibdeps` + `fcitx5 | ibus`; both shells in
  one package, as `fcitx5-chewing` + `ibus-chewing` come from one source; not
  `cargo-deb` — a second asset list would drift from `make install`) — on the
  GitHub-hosted Ubuntu runner (`.github/workflows/linux-build.yml`, mirror of
  `windows-build.yml`: built on every PR; a `main` `workflow_dispatch` from
  `tools/release/stage-desktop.sh` attaches the `.deb` + SHA-256 to the `desktop-<version>` DRAFT,
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
  and on dispatch) adds what the Mac cannot: a real
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
  handling — is left to device dogfood (`dogfood-checklist.md`).
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
| Appearance rows Candidate Text Size / Candidate Window Size / Font | yes | yes | hidden (panel-owned) | unsupported host capability |
| Manage Fonts pane | selects the candidate typeface | selects the candidate typeface | hidden (panel-owned; font set in Fcitx5 / IBus) | unsupported host capability |
| Focus loss mid-composition | client commits | text stays as host left it | daemon commits (`PREEDIT_COMMIT`) | platform-adapted |
| Chinese/English Shift tap | none (OS switches sources) | yes | none (IBus switches engines) | identical to macOS |
| Global chords | Carbon hotkeys, session-scoped | preserved keys + fallback | matched in `ProcessKeyEvent` while active | identical semantics |
| Mode flash HUD | yes | yes | panel property symbol / label | platform-adapted |
| Menu | input-source menu | tray button menu | panel property menu | platform-adapted |
| Update check | Sparkle-style in-app | scheduled task + in-app | none; version + download link (§ L10) | named divergence: the package manager updates a Linux input method |
| Data dir | `~/Library/Application Support/<bundle>` | `%APPDATA%\TaigiKeyboard` | XDG config + data split | platform-adapted |
| Settings frame persisted | yes | no | no | named divergence |
| MOE dictionary off: its eleven accent rows | shown, greyed | shown, greyed | shown, greyed (mirrors the other desktops) | identical semantics |
| Sidebar icons | SF Symbols | Fluent | Adwaita symbolic (`preferences-system`, `applications-graphics`, `input-keyboard`, `emblem-documents`, `x-office-address-book`, `document-open-recent`, `help-about`) | identical semantics |
| Custom Dictionary table | two columns Romanization / Hanji, double-click edits | two columns, ✎ edits | two columns, double-click or ✎ edits | identical semantics |
| Release flow | `make desktop-release` stages the `.pkg` | the `.exe` on the same draft | the `.deb` on the same draft (`linux-build.yml` `attach`); announce writes `_data/linux_release.json` (download buttons only, no appcast, § L10) | identical semantics |

## Best practices alignment (references)

| Mainstream practice | Source | This plan (phase) |
|---|---|---|
| Engine as a separate process the daemon spawns from a component XML; panel draws preedit + candidates | ibus `src/ibusfactory.c`, `src/ibusengine.c`; rime `ibus-rime`, mozc `unix/ibus/` | L1, L4 |
| Serialisable wire tuples `(sa{sv}…)` in a fixed field order kept for compatibility | ibus `src/ibusserializable.c:277`, `src/ibusenginedesc.c` ("The serialized order should be kept") | L1 |
| Shell-independent core tested natively, thin platform shells | `windows-roadmap.md` W1, `references/ChiaKey` `ChiaKeyCore` facade | L2, L13 |
| Blind authoring gated by cross-target checks + hosted runner | `windows-roadmap.md` W13 | L12 |
| XDG base directories for config vs data | freedesktop basedir spec; `ibus` itself (`g_get_user_config_dir()` for its bus file, `src/ibusshare.c:195`) | L7 |
| No self-update check in an input method; the distribution updates the package | distro IBus / Fcitx5 engines (`ibus-rime`, `ibus-mozc`, `fcitx5-chewing`) never check | L10 |
| libadwaita preference widgets for an IME's settings | GNOME Settings, `ibus-setup` (GTK), `ibus-anthy` setup dialog | L3 |
| Lazy engine init on first consumed key | rakukan `factory.rs:413-419`; Windows `Runtime::prepare_for_first_key` | L13 |

**Deliberately not adopted**: an own GTK candidate popup (unpositionable under Wayland;
no IBus engine does it); libibus FFI bindings (C toolchain on the host, GObject
ownership across FFI, and nothing the D-Bus surface lacks); Qt settings window (second toolkit; the IBus panel is GTK);
any update check, manual or automatic, and a build flag to switch one off (§ L10: nothing to switch); Flatpak (cannot host an IBus engine); a `linux/` crate
that depends on `../windows/crates/…` (wrong shape, § L2). **YAGNI**: surrounding-text
capability (nothing in the key table reads the document); handwriting methods answer
`()`; `ForwardKeyEvent` is unused (a key the engine does not consume is answered `false`
and the client processes it itself).
