# ChromeOS Release

> **Type**: Release flow
> **Keywords**: `chromeos`, `chrome web store`, `extension`, `input method`
> **Status**: Draft 2026-11-08, nothing published. Plan and decisions: `chromeos-roadmap.md`. Publishing is the maintainer's.

The ChromeOS input method ships as one Chrome Web Store item, an MV3 extension built from `web/chromeos/` over the web version's wasm. On a Chromebook the store's "Add to Chrome" is the installer; the store updates it.

## Build

```
make -C web chromeos        # → web/target/taigi-keyboard-chromeos.zip (~16 MB)
```

It builds the wasm from source, so there is no stale-artifact step. `make web-check` (CI) already runs the ChromeOS Node tests; the zip itself is built only by hand.

Before an upload, set `version` in `web/chromeos/manifest.json`: the store refuses a version that is not higher than the last one it took.

## First upload

1. Chrome Web Store developer account (one-time registration fee) under the maintainer's Google account.
2. Developer Dashboard → New item → upload the zip.
3. Store listing, privacy and distribution as below, then submit for review.

### Store listing

| Field | Value |
|---|---|
| Name | 台語齒盤 (from the manifest) |
| Category | Tools |
| Language | Chinese (Traditional) |
| Icon | `icons/128.png` in the zip (the iOS app icon) |
| Screenshots | 1280×800, at least one; see below |

The listing says plainly that the item is for ChromeOS: on Chrome for Windows, macOS or Linux there is no `chrome.input.ime`, so it installs and does nothing. For those systems the listing links the desktop apps and the web version.

### Privacy practices tab

| Field | Value |
|---|---|
| Single purpose | A Taiwanese (Taigi) input method for ChromeOS |
| `input` | Registers the input method and receives its keys |
| `offscreen` | Keeps the input method's worker running between keystrokes; a key does not wake a stopped worker (`chromeos-roadmap.md` § Spike result) |
| `storage` | Keeps the settings, synced through the user's Google account (`chrome.storage.sync`) |
| Remote code | None: the wasm is in the package |
| Data collected | None. Keys are handled on the device; settings sync through Chrome; learned words stay in the extension's IndexedDB. Nothing is sent to any server of ours |

### Screenshots

The candidate window over a text field, and the settings page (Display Language 漢字). Drafts were taken in the development VM at 1280×800 (Xvfb `-screen 0 1280x800x24`, `--ash-host-window-bounds=1280x800`), but `linux-chromeos` keeps a stub "sign-in error" notification on screen and its profile reports `navigator.language` `en-US`, so the settings page shows English unless Display Language is set. Take the published ones on a real device in Phase 6.

## After publishing

- Users add it in Settings → Keyboard and inputs → Input methods → Add input methods. The manifest files it under Chinese (Traditional, Taiwan) (`language: zh-TW`); where ChromeOS shows it in that list is not verified on a real device.
- Phase 6 of the roadmap (real ChromeOS, Flex from USB) checks the published build.

## Timing

Maintainer 2026-11-08: "no rush to release; handle it when releasing v3.9.0 together with the other desktops". The first upload goes out with the desktop v3.9.0 release, as version 3.9.0.

## Open (maintainer)

- Whether the website's download page gets a ChromeOS link.
- Store listing description text (short and neutral, as the other stores').
