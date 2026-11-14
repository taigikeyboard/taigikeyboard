# macOS release — signed, notarized, web-distributed

macOS input methods cannot ship on the Mac App Store: an App Sandbox process
cannot register the system-wide mach service that InputMethodKit requires of an
input-method server, and Apple's developer support has confirmed the category is
out of scope for the store. The only distribution route is a Developer ID
installer package downloaded from the web, which is what
`.github/workflows/macos-build.yml` produces on a GitHub-hosted Mac.

iOS is unaffected — it continues to ship through App Store Connect.

**Entry point for a desktop release is [`desktop-release.md`](desktop-release.md)** (one version, one draft GitHub release holding every desktop platform's installers, `make desktop-release`); this file is the macOS-specific half it drives.

## Architectures

One universal `.pkg`, not one download per Mac. Every layer carries both
architectures:

| Layer | Shape |
|---|---|
| Rust engine | `RustTaigi.xcframework` — **one** `macos` slice, `macos-arm64_x86_64`. Built as two thin archives and `lipo`-ed together: arm64 and x86_64 macOS are the same *platform*, and `xcodebuild -create-xcframework` rejects two libraries that resolve to it ("represent two equivalent library definitions"). |
| App executable | One `swift build --arch <arch>` per architecture, then `lipo -create`. Deliberately not the Swift Build backend's multi-architecture mode, which SwiftPM documents for universal binaries but which cannot link this package — it drops the `@_cdecl` logger-sink symbols the Rust archive imports and fails for *both* architectures. Since Swift 6.4 (Xcode 27) that backend is the default even for a single `--arch`, so `bundle-app.sh` passes `--build-system native` explicitly. vChewing builds per-architecture for the same reason. |
| Installer | `hostArchitectures="arm64,x86_64"` in the generated distribution. |

`macos/rust-toolchain.toml` declares both Apple desktop targets, so the
prerequisite is a checkout concern rather than something a build script installs
behind the developer's back.

**Debug stays native.** `bundle-app.sh debug` builds only for the host
architecture — recompiling the whole dependency graph for an architecture this
Mac cannot execute costs every dev-loop iteration and catches nothing.
`release-app.sh` refuses a bundle without both slices, so the shipping path
cannot take the native branch.

Three assertions pin the contract, each an exact set rather than a
"contains" check, because a bundle that silently lost x86_64 still contains
arm64 and only an Intel Mac would ever find out:

- the xcframework declares exactly one `macos` slice carrying exactly
  `arm64,x86_64`;
- the assembled executable's `lipo -archs` is exactly `arm64,x86_64` (release)
  or exactly `uname -m` (debug);
- each slice's `LC_BUILD_VERSION` `minos` equals `LSMinimumSystemVersion`. The
  Rust builds set `MACOSX_DEPLOYMENT_TARGET=14.0` explicitly to make that true —
  rustc's per-target default is lower and differs between the two.

## One-time account setup

Everything here is done once per account, on the maintainer's Mac; the result
goes into the `release` environment (*3. The release environment* below), and
the release runner never touches a person's keychain. `release-app.sh` fails
fast with the exact remedy when a piece is missing.

### 1. Two certificates

Web distribution needs both halves of Developer ID, and they are different
certificates:

| Certificate | Signs | Created in |
|---|---|---|
| Developer ID Application | the `.app` bundle | Xcode → Settings → Accounts → Manage Certificates → **+** |
| Developer ID Installer | the `.pkg` | same panel |

Confirm both landed in the login keychain:

```sh
security find-identity -v | grep "Developer ID"
```

`release-app.sh` resolves each certificate itself and stops if more than one
is valid — which happens while a renewed certificate overlaps the one it
replaces, since both carry the same name. Name the one to sign with through
`DEVELOPER_ID_APPLICATION` / `DEVELOPER_ID_INSTALLER` when that is the case.

Back up the private keys (Keychain Access → export as `.p12`). A lost Developer
ID key cannot be re-issued for the same certificate, and Apple caps how many of
each type an account may hold.

### 2. An App Store Connect API key

Notarization authenticates separately from code signing. App Store Connect →
Users and Access → Integrations → **Team Keys** → **+**, access *Developer*.
Download the `.p8` (offered once) and note the key ID and the issuer ID shown
above the list. `release-app.sh` uses it when `NOTARY_API_KEY` (the `.p8`
path), `NOTARY_API_KEY_ID` and `NOTARY_API_ISSUER` are set — `notarytool
--key / --key-id / --issuer`.

A local, unpublished build may still notarize through a stored keychain profile
(`NOTARY_PROFILE`, default `TaigiKeyboard`), used when `NOTARY_API_KEY` is unset:

```sh
xcrun notarytool store-credentials "TaigiKeyboard" \
  --apple-id <apple-id> \
  --team-id <team-id> \
  --password <app-specific-password>
```

### 3. The release environment

The release runner reads everything from the repository's `release`
environment (only `main` and the release tags may use it):

| Name | Kind | Value |
|---|---|---|
| `MACOS_DEVELOPER_ID_APPLICATION_P12` | secret | the Developer ID Application certificate + key, exported as `.p12`, base64 (`base64 -i app.p12 \| pbcopy`) |
| `MACOS_DEVELOPER_ID_INSTALLER_P12` | secret | the same for Developer ID Installer |
| `MACOS_P12_PASSWORD` | secret | the export password of both `.p12` |
| `APPLE_API_KEY_P8` | secret | the `.p8` file's PEM text (`gh secret set APPLE_API_KEY_P8 --env release < AuthKey_<id>.p8`) |
| `APPLE_API_KEY_ID` | variable | the key ID |
| `APPLE_API_ISSUER_ID` | variable | the issuer ID |

Set with `gh secret set <name> --env release` / `gh variable set <name> --env
release`. A renewed certificate is a new `.p12` in the same secret; the runner's
keychain only ever holds the one in the environment, so the renewal overlap
described above cannot happen there.

## Cutting a release

The monthly cut (`desktop-train.yml`) calls `macos-build.yml`'s build for the
staged commit and signs the package in its own `sign-and-beta` job (composite `macos-sign-attach`, shared
with `macos-build.yml`); a hotfix (`make desktop-patch PLATFORM=macos`) or a
manual full stage (`make desktop-release`) on any machine dispatches
`.github/workflows/macos-build.yml` with the staged commit
(`desktop-release.md`). Two jobs on two `macos-26` runners, Xcode pinned
through `DEVELOPER_DIR` (an image update cannot change the SDK a release links):

| Job | Has | Runs |
|---|---|---|
| `build` | no secret, no environment, a read-only token, none on disk | checks out the staged commit; `make macos-libs` (the universal engine xcframework — generated and gitignored, so always built from the commit, [`build-artifacts.md` § A release rebuilds first](build-artifacts.md#a-release-rebuilds-first)); `bundle-app.sh release` (ad-hoc signed); checks the tree is unchanged; hands the bundle on as an artifact kept 30 days (as long as an approval may wait) |
| `release` | the `release` environment: both `.p12`, the `.p8`, a write token | on a fresh VM, compiles nothing: before the checkout, refuses a staged commit that is not on `main` (its scripts will hold the keys); joins the draft (`join_desktop_draft`); imports both `.p12` (non-exportable) into a temporary keychain with Apple's two Developer ID intermediates and writes the `.p8`; `release-app.sh` with no GitHub token; removes the keychain and the key; then `publish-release.sh` with the token, which joins the draft again — refusing one a newer staging run re-created on another commit, never moving it — and attaches the package |

The `release` job's steps are the composite action
`.github/actions/macos-sign-attach`. The monthly train (`desktop-train.yml`)
calls this workflow with `build_only` and runs the same action in its
`sign-and-beta` job, so a cut asks for one approval for signing and the beta
together.

**Why two jobs:** the build executes third-party code — every crate's build
script and proc macro, every SwiftPM package. In one job, such code could leave
a process behind that outlives its step and waits for the keychain or the
`.p8`. On separate machines, nothing that ran there ever coexists with a
credential; the signing machine runs only Apple's tools, this repository's
scripts and two SHA-pinned GitHub actions. It signs the bundle `build` produced
— that is the product, and notarization scans it — but executes none of it.

A rejected notarization keeps its log as the `macos-notarization` artifact.

Re-staging the same commit while an earlier macOS run is still building fails
the later run's read-back (a re-signed package has other bytes than the one
already attached); wait for the earlier run to finish, then stage again — the
same recovery as the Linux half.

A pull request touching the workflow or the release scripts runs the `build`
job alone.

What `release-app.sh` does, in order — it builds nothing:

1. **Preflight** — clean working tree, each certificate resolvable to exactly
   one fingerprint, notarization credential authenticates.
2. **Sign** — the release bundle `bundle-app.sh release` assembled
   (`macos/.build/bundle/`, refused unless it carries both slices) is re-signed
   with the Developer ID Application identity, the hardened runtime and a
   secure timestamp.
3. **Verify the signature** — signed by a Developer ID Application certificate,
   hardened-runtime flag present, secure timestamp present, no entitlements.
   All four are notarization prerequisites, and catching them here costs
   seconds instead of a failed round trip.
4. **Package** — `pkgbuild` against a staged root holding only the `.app`,
   plus a `postinstall` script, then `productbuild --sign <Developer ID
   Installer>` to wrap it in a product archive carrying the install domain
   (see *Where it installs* below). `pkgutil --check-signature` then confirms
   the archive really carries a Developer ID Installer signature from the same
   team that signed the bundle.
5. **Notarize and staple** — submit, wait, staple the ticket onto the package,
   then `stapler validate` + `spctl --assess --type install`.
6. **Name the output** — move the finished package to
   `macos/.build/distribution/TaigiKeyboard-<version>.pkg` only after every check
   passed, and print its SHA-256.
Staging is `macos/scripts/publish-release.sh`, the workflow's next step
(*Publishing the package* below). It picks up only that exact name, so a
throwaway package can never be staged.

Flags:

The release runner passes none: a fresh runner has no package from a previous
attempt to `--force` over. A local throwaway build:

```sh
make macos-libs && bash macos/scripts/bundle-app.sh release
bash macos/scripts/release-app.sh --skip-notarize   # packaging only, unshippable
bash macos/scripts/release-app.sh --allow-dirty     # package from a dirty tree
```

`--skip-notarize` and `--allow-dirty` both stamp the reason into the output
filename, so an unpublishable package cannot be confused for a release.

## Versioning

`macos/App/Info.plist` is the single source of truth and the release script
only reads it: `CFBundleShortVersionString` names the download, and
`CFBundleVersion` is what Installer compares between packages. Neither is
edited by hand —
[`manual-release-notes.md` § Set the version](manual-release-notes.md#set-the-version).

## Where it installs

Into `~/Library/Input Methods`, **without an administrator password**.

The mechanism is indirect. `pkgbuild` is given an absolute
`--install-location /Library/Input Methods`, and the generated distribution then
enables only the current-user-home domain:

```xml
<domains enable_currentUserHome="true" enable_localSystem="false" enable_anywhere="false"/>
<pkg-ref id="…" version="…" auth="none">component.pkg</pkg-ref>
```

The domain is what does the work. A home-domain install runs as the installing
user and cannot write outside that home, so it needs no authorization, and the
Installer rebases the component's path under the home — the absolute path
recorded in the component (`auth="root"` and all) is never where files land.

`pkg-ref auth` is deprecated and no longer decides this. `auth="none"` is kept
anyway because vChewing — the closest peer, also signed and notarized into the
home domain — still sets it, and an inert attribute is cheaper than finding out
some macOS version still reads it.

`<domains>` is the only reason the pipeline runs `productbuild` at all: it
exists in a distribution, not in a component package.

⚠ Verified from the built archive, not from an install: an actual signed
double-click install is what finally proves no password prompt appears.

Consequence: the input method is installed for the account that ran the
installer, not for every account on the Mac. Another user on the same Mac
installs it again. For an input method — a personal setting, not a system
component — that is the trade most macOS IMEs make.

A local `make install` build lives in the same directory, so a package install
simply upgrades it. No uninstall step is needed first.

## Publishing the package

The desktop-wide flow, what staging checks on the package (stapled, Gatekeeper,
its declared identity) and why the release lives in this repository:
[`desktop-release.md`](desktop-release.md).

## Notes

- **No entitlements file.** The bundle is not sandboxed, links the Rust engine
  statically, and uses no JIT or dynamic-library exemption, so the hardened
  runtime needs nothing declared. azooKey-Desktop's mach-registration
  entitlement exists only because that app *is* sandboxed; it does not transfer.
- **No `codesign --deep`** when signing. The bundle has no nested code to sign:
  `bundle-app.sh` assembles it from one executable plus data files, and checks
  that executable links nothing through `@rpath`. `--deep` would only mask a
  nested-signing mistake if one were ever introduced. It is used for
  *verification* only.
- **`productbuild` only for `<domains>`.** One component and no installer
  choices, so the distribution declares `customize="never"` and adds nothing
  else. Every value in it is generated from `Info.plist`, so it cannot drift
  from the bundle it describes.
- **A `postinstall` script that only kills the running process.** The installer
  replaces the bundle underneath a live input method, which would otherwise keep
  serving keystrokes from the version it just overwrote. The process relaunches
  on the next keystroke, so nothing needs starting.
- **Only the package is notarized.** It is the only artifact distributed, and
  stapling it is what lets Gatekeeper clear the install without network access.
- **No Sparkle.** The app's own update check fetches the manifest; from the
  settings window it downloads `packageURL` and opens it in Installer
  (`macos/updates/README.md`, `UpdateChecker.swift`).

## Alignment with other macOS input methods

Read from the local clones under `references/`, not from release pages:

| Input method | Artifact | Installs to | Administrator password |
|---|---|---|---|
| vChewing (`vChewing-macOS/BuildPKG.sh`) | `.pkg`, signed + notarized | `~/Library/Input Methods` | no |
| MacishType (`MacishType/macos/Makefile`) | `.pkg`, unsigned | `~/Library/Input Methods` | no |
| azooKey-Desktop (`azooKey-Desktop/pkgbuild.sh`) | `.pkg`, signed + notarized | `/Library/Input Methods` | yes |
| McBopomofo (`McBopomofo/Source/Installer/`) | custom installer `.app` | `~/Library/Input Methods` | no |
| khiin-rs (`khiin-rs/swift/osx/build.sh`) | none — dev script copies the `.app` | `~/Library/Input Methods` | no |

`.pkg` is what every one of them that ships an installer file uses; none ships a
`.dmg` or a bare `.zip`, because an input method has to land in a specific
directory rather than be dragged to `/Applications`.

Borrowed deliberately:

| Practice | Source | Where |
|---|---|---|
| `pkgbuild --analyze`, then flip `BundleIsRelocatable` off | `MacishType/macos/Makefile:188-190` | `release-app.sh` component plist |
| `<domains enable_currentUserHome>` for a no-password install | `vChewing-macOS/BuildPKG.sh` distribution, `MacishType/macos/Makefile:208` | `release-app.sh` distribution |
| `auth="none"` on the `pkg-ref` | `vChewing-macOS/BuildPKG.sh` distribution | `release-app.sh` distribution |
| `postinstall` that kills the running input method | `MacishType/macos/Makefile:177` | `release-app.sh` postinstall |
| `notarytool submit --wait` then `stapler staple` | `azooKey-Desktop/pkgbuild.sh`, `vChewing-macOS/BuildPKG.sh` | `release-app.sh` notarization |
| `<volume-check>` against the bundle's minimum OS | `vChewing-macOS/BuildPKG.sh` distribution | `release-app.sh` distribution |

Deliberately not adopted:

| Practice | Source | Why not |
|---|---|---|
| `productsign` as a separate step | `azooKey-Desktop/pkgbuild.sh` | `productbuild --sign` already signs the archive it produces |
| Separate notarization of the `.app` | `azooKey-Desktop/pkgbuild.sh` | the package is the only artifact distributed |
| System-wide `/Library/Input Methods` | `azooKey-Desktop/pkgbuild.sh` | costs an administrator password for a per-user setting |
| Localized installer welcome / licence / conclusion panes | `vChewing-macOS/BuildPKG.sh` | product copy, and none of it is written yet |
| `onConclusionScript` re-login prompt | `MacishType/macos/Makefile:220` | the input-source cache note lives in the install instructions (`docs/INSTALL.md`) instead |
| `codesign --deep` when signing | `vChewing-macOS/BuildPKG.sh` | the bundle has no nested code; see above |
