# ChromeOS Release

> **Type**: Release flow
> **Keywords**: `chromeos`, `chrome web store`, `extension`, `input method`
> **Status**: Active. On the desktop train; 3.9.0, the first version, was uploaded by hand, and every later one is uploaded and submitted by the announcement (§ Upload). Plan and decisions: `chromeos-roadmap.md`.

The ChromeOS input method ships as one Chrome Web Store item, an MV3 extension built from `web/chromeos/` over the web version's wasm. On a Chromebook the store's "Add to Chrome" is the installer; the store updates it.

## On the desktop train

ChromeOS is a desktop platform like macOS, Windows and Linux (USER 2026-11-12): one version, one draft, one beta, one publish ([`desktop-release.md` § The flow](desktop-release.md#the-flow)).

| Step | What happens to ChromeOS |
|---|---|
| `make version-desktop <v>` | Writes `version` in `web/chromeos/manifest.json` with the rest of the train; `check-versions --train desktop` holds it there. The store refuses a version that is not higher than the last one it took, which the train's downgrade guard already enforces |
| The monthly cut (`desktop-train.yml`, or `make desktop-release` by hand) | Runs `.github/workflows/chromeos-build.yml`, which runs `make -C web chromeos` on a hosted runner and attaches `TaigiKeyboard-<v>-chromeos.zip` + `.sha256` to the draft |
| `make desktop-patch PLATFORM=chromeos` | A ChromeOS-only patch: its own version, only the zip on the draft |
| Beta | The zip's R2 link is in the Discord beta post with the other installers. Testers unpack it and use **Load unpacked** in `chrome://extensions` (Developer mode). An unpacked copy has its own extension ID, separate from the store item, so its settings and learned words are separate too, and it never updates itself |
| Publish → announce | Mirrors the zip to `dl.taigikeyboard.tw/desktop/`, writes `_data/chromeos_release.json`, then uploads the zip to the Chrome Web Store and submits it for review (§ Upload) |

Every pull request that runs `web.yml` also builds the zip (`make -C web chromeos` after `make -C web test smoke`); dispatching `chromeos-build.yml` without `source_sha` builds it without attaching. To build locally:

```
make -C web chromeos        # → web/target/taigi-keyboard-chromeos.zip (~16 MB)
```

It builds the wasm from source, so there is no stale-artifact step. `make web-check` (CI) runs the ChromeOS Node tests.

## Upload

Automatic (maintainer 2026-11-16: upload the zip only; the listing text, screenshots and privacy fields stay as set in the Dashboard). When a published release carries `TaigiKeyboard-<v>-chromeos.zip` — the monthly publish or a `make desktop-patch PLATFORM=chromeos` — `desktop-announce.yml`, after the website and the Homebrew cask, mints a store token (§ One-time below) and runs `tools/release/chrome-web-store.sh submit chromeos <v> <sha256>` (the browser extension's zip, the same with `extension`: `browser-extension.md` § Release):

1. reads the item's status; stops if `<v>` is already submitted or published (a re-run is safe), fails if the store holds a newer version, and fails if another version is still in review or staged — let it finish (a staged one: publish it) or cancel it in the Dashboard, then re-run the workflow (it is not cancelled for you; a re-run repeats the whole announcement, ~30 min, before the submit)
2. downloads the zip from the release anonymously and checks it against the announcement's sha256
3. uploads it (`…/upload/v2/publishers/<publisher>/items/<item>:upload`), waits while the upload is in progress, and checks the version the store read from the manifest when the upload response carries it
4. submits it (`:publish`, `DEFAULT_PUBLISH`): the store reviews it and publishes it on approval

The Discord success note names the state the store returned (normally `PENDING_REVIEW`). A failure posts to #ci-alerts; the website and the cask are already done by then, and the Homebrew cask audit still runs.

By hand (a failed job, or with the job's token expired): `make desktop-announce` prints the `submit` command; run it with the `CWS_*` variables of `chrome-web-store.sh`'s header. Or upload in the Developer Dashboard: the item → Package → Upload new package → Submit for review.

### One-time: the Chrome Web Store API

The Chrome Web Store API v2 accepts a service account ([Use the Chrome Web Store API](https://developer.chrome.com/docs/webstore/using-api), [service accounts](https://developer.chrome.com/docs/webstore/service-accounts)). GitHub Actions reaches it keyless: OIDC token → Workload Identity Federation → a short-lived service-account token scoped to `https://www.googleapis.com/auth/chromewebstore`. The store's docs omit federation; `release-identity.yml`'s `chrome-web-store` job checks it. The calls themselves are `tools/release/chrome-web-store.sh`. The org's allowed-actions list names `google-github-actions/auth@*` (2026-11-16).

In the Google Cloud console or `gcloud`, with `PROJECT_ID` a project of the maintainer's:

1. Enable the Chrome Web Store API, the IAM API (`iam.googleapis.com`, for the setup), the IAM Service Account Credentials API (`iamcredentials.googleapis.com`) and the Security Token Service API (`sts.googleapis.com`).
2. Create a service account with no roles, e.g. `chrome-web-store@PROJECT_ID.iam.gserviceaccount.com`.
3. Create a workload identity pool `github` and in it an OIDC provider `taigikeyboard`, issuer `https://token.actions.githubusercontent.com`:
   - attribute mapping `google.subject=assertion.sub,attribute.repository_id=assertion.repository_id`
   - attribute condition `assertion.repository_id == '1359878131' && assertion.repository_owner_id == '241070258' && assertion.environment == 'release'` — numeric ids survive a rename and cannot be reclaimed by a new repository of the same name; the `release` environment admits only `main` and release tags (`desktop-release.md` § One-time: the release App)
4. On the service account, grant `roles/iam.workloadIdentityUser` to `principalSet://iam.googleapis.com/projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github/attribute.repository_id/1359878131`.
5. Developer Dashboard → Account: add the service account's email (one per publisher, so it covers both items: this one and the browser extension). The publishing Google account needs 2-Step Verification.
6. Repository Settings → Environments → `release` → Variables (identifiers, not secrets):

   | Variable | Value |
   |---|---|
   | `CWS_WIF_PROVIDER` | `projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github/providers/taigikeyboard` |
   | `CWS_SERVICE_ACCOUNT` | the service account's email |
   | `CWS_PUBLISHER_ID` | Developer Dashboard → Publisher → Settings |
   | `CWS_ITEM_ID` | the ChromeOS item's 32-letter id (its Package tab) |
   | `CWS_EXTENSION_ITEM_ID` | the browser extension item's id (`macjldjheiajcimfcjdcegagaemcelkf`) |

7. `gh workflow run release-identity.yml` and approve the `release` environment (by hand: `CWS_ACCESS_TOKEN=$(gcloud auth print-access-token --impersonate-service-account=<email> --scopes=https://www.googleapis.com/auth/chromewebstore) CWS_PUBLISHER_ID=… CWS_ITEM_ID=… tools/release/chrome-web-store.sh status chromeos`, which needs `roles/iam.serviceAccountTokenCreator` on the service account for your account). The `chrome-web-store` job prints each item's published and submitted versions; it uploads nothing. A failed token mint points at steps 1–4; a 403 from the store at the Chrome Web Store API (step 1) or step 5; a 404 at the two ids in step 6. The job prints Google's error body.

## Store item

Created by hand under the maintainer's developer account for 3.9.0. The announcement never touches the fields below; they are the record of what the Dashboard holds.

### Store listing

| Field | Value |
|---|---|
| Name | 台語齒盤：ChromeOS 台語輸入法 (manifest `name`, zh_TW); "ChromeOS" sets it apart from a possible browser extension, which would be its own store item. The options page `<title>` keeps the short 台語齒盤 ChromeOS, and the input method list entry (`input_components[].name`) stays 台語齒盤 |
| Summary | 免費、開源、跨裝置 ê 台灣話拍字家私 (manifest `description`, which the store shows as the summary) |
| Translations | Name and summary come from `web/chromeos/_locales/{zh_TW,en,ja}/messages.json` (`default_locale` zh_TW); the store and Chrome pick by the user's language. English and Japanese follow the website and `i18n/` wording |
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
| `notifications` | Shows the input mode (Taigi / English, Hanji / Romanization) for 2 s at the bottom right when it switches |
| `offscreen` | Keeps the input method's worker running between keystrokes; a key does not wake a stopped worker (`chromeos-roadmap.md` § Spike result) |
| `storage` | Keeps the settings, synced through the user's Google account (`chrome.storage.sync`) |
| Remote code | None: the wasm is in the package |
| Data collected | None. Keys are handled on the device; settings sync through Chrome; learned words stay in the extension's IndexedDB. Nothing is sent to any server of ours |

### Screenshots

The candidate window over a text field, and the settings page (Display Language 漢字). Drafts were taken in the development VM at 1280×800 (Xvfb `-screen 0 1280x800x24`, `--ash-host-window-bounds=1280x800`), but `linux-chromeos` keeps a stub "sign-in error" notification on screen and its profile reports `navigator.language` `en-US`, so the settings page shows English unless Display Language is set. Take the published ones on a real device in Phase 6.

## After publishing

- Users add it in Settings → Keyboard and inputs → Input methods → Add input methods. The manifest files it under Chinese (Traditional, Taiwan) (`language: zh-TW`); where ChromeOS shows it in that list is not verified on a real device.
- Phase 6 of the roadmap (real ChromeOS, Flex from USB) checks the published build.
