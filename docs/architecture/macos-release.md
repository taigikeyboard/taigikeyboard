# macOS release — signed, notarized, web-distributed

macOS input methods cannot ship on the Mac App Store: an App Sandbox process
cannot register the system-wide mach service that InputMethodKit requires of an
input-method server, and Apple's developer support has confirmed the category is
out of scope for the store. The only distribution route is a Developer ID
installer package downloaded from the web, which is what `make macos-release`
produces.

iOS is unaffected — it continues to ship through App Store Connect.

## Architectures

One universal `.pkg`, not one download per Mac. Every layer carries both
architectures:

| Layer | Shape |
|---|---|
| Rust engine | `RustTaigi.xcframework` — **one** `macos` slice, `macos-arm64_x86_64`. Built as two thin archives and `lipo`-ed together: arm64 and x86_64 macOS are the same *platform*, and `xcodebuild -create-xcframework` rejects two libraries that resolve to it ("represent two equivalent library definitions"). |
| App executable | One `swift build --arch <arch>` per architecture, then `lipo -create`. Deliberately not the Swift Build backend's multi-architecture mode, which SwiftPM documents for universal binaries but which cannot link this package — it drops the `@_cdecl` logger-sink symbols the Rust archive imports and fails for *both* architectures. vChewing builds per-architecture for the same reason. |
| Installer | `hostArchitectures="arm64,x86_64"` in the generated distribution. |

`engine/rust-toolchain.toml` declares both Apple desktop targets, so the
prerequisite is a checkout concern rather than something a build script installs
behind the developer's back.

**Debug stays native.** `bundle-app.sh debug` builds only for the host
architecture — recompiling the whole dependency graph for an architecture this
Mac cannot execute costs every dev-loop iteration and catches nothing.
`release-app.sh` always passes `release`, so the shipping path cannot take the
native branch.

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

Everything here is done once per machine. `make macos-release` fails fast with
the exact remedy when a piece is missing.

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

`make macos-release` resolves each certificate itself and stops if more than one
is valid — which happens while a renewed certificate overlaps the one it
replaces, since both carry the same name. Name the one to sign with through
`DEVELOPER_ID_APPLICATION` / `DEVELOPER_ID_INSTALLER` when that is the case.

Back up the private keys (Keychain Access → export as `.p12`). A lost Developer
ID key cannot be re-issued for the same certificate, and Apple caps how many of
each type an account may hold.

### 2. A stored notarization credential

Notarization authenticates separately from code signing. Store it once under the
profile name the release script expects:

```sh
xcrun notarytool store-credentials "TaigiKeyboard" \
  --apple-id <apple-id> \
  --team-id <team-id> \
  --password <app-specific-password>
```

The password is an **app-specific password** from appleid.apple.com, not the
Apple ID password. Override the profile name with `NOTARY_PROFILE` if an
existing profile should be reused.

## Cutting a release

```sh
make build          # only when engine/ or dictionary/ sources moved
make macos-release
```

`make macos-release` deliberately does not rebuild the Rust engine or the
dictionary. Both platforms link pre-built artifacts that are committed to the
repository, so a release from a clean tree ships exactly what is committed —
but nothing can prove those committed artifacts were regenerated from the
committed sources. That is what the stale-binary gate in `CLAUDE.md` is for, and
it stays a manual step.

What the script does, in order:

1. **Preflight** — clean working tree, each certificate resolvable to exactly
   one fingerprint, notarization credential authenticates.
2. **Build and sign** — `scripts/bundle-app.sh release --sign <Developer ID
   Application>`, which runs every assembly check the dev loop runs and then
   signs with the hardened runtime and a secure timestamp.
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
7. **Publish** — hand the package to `macos/scripts/publish-release.sh` (see
   *Publishing the package* below).

Flags:

`make macos-release` passes `--force --publish`, so cutting a release needs no
flags: it publishes, and it overwrites the local package left by a previous
attempt at the same version. Re-cutting a version is the normal case — a release
flow is verified by running it — and both defaults exist so that verifying it
twice takes the same command as verifying it once.

`--publish` refuses to run alongside either throwaway flag, and the refusal comes
before the build rather than after the notarization wait. Those builds therefore
bypass the make target:

```sh
bash macos/scripts/release-app.sh --skip-notarize   # packaging only, unshippable
bash macos/scripts/release-app.sh --allow-dirty     # build from a dirty tree
```

`RELEASE_FLAGS` still reaches the script for anything else, but passing either of
the two above through it fails on purpose.

`--skip-notarize` and `--allow-dirty` both stamp the reason into the output
filename, so an unpublishable package cannot be confused for a release.

## Versioning

`macos/App/Info.plist` is the single source of truth, and the release script
only reads it:

| Key | Role |
|---|---|
| `CFBundleShortVersionString` | marketing version; names the download |
| `CFBundleVersion` | package version `pkgbuild --version` compares between releases |

The Installer decides upgrade-versus-downgrade from `CFBundleVersion`, so it has
to increase on every published package even when the marketing version does not.

Neither moves on its own — release scope and timing are the maintainer's call —
but neither is edited by hand either. `make version-desktop x.y.z` writes both
keys here and the matching Windows version in the same pass, so the two desktop
platforms cannot drift apart; `python3 tools/release_notes.py check-versions
--train desktop --version x.y.z` is the gate that proves they did not. iOS and
Android are the separately numbered mobile train, and nothing compares the two
trains.

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

A desktop release happens in two halves with a manual test between them, and
**nothing reaches a user until a person publishes it**:

| | Runs | Does |
|---|---|---|
| Stage the package | `make macos-release` (this Mac) | Builds, signs, notarizes, packages, then `macos/scripts/publish-release.sh` puts the `.pkg` on a **draft** release `desktop-<version>` |
| Stage the installer | `make windows-release RELEASE_FLAGS=--skip-sign` (the Windows box) | Attaches the `.exe` to the same draft |
| **Test** | the maintainer | `gh release download desktop-<version> --repo taigikeyboard/taigikeyboard --dir ~/Downloads`, install, use both |
| **Publish** | the maintainer | `gh release edit desktop-<version> --repo taigikeyboard/taigikeyboard --draft=false`, or the web UI. This is what creates the tag |
| Announce | `make desktop-announce` (either machine) | Proves both downloads are anonymously reachable, writes both `_data/*_release.json`, waits for the live appcasts |

Staging creates no tag — publishing does — and a draft has no public asset URL,
so no user, no search engine and no installed copy can reach what is staged.
(A tag pushed by hand beforehand is accepted, as long as it names the commit
being staged.) Between the two halves, the `.sha256` receipt staged beside each
installer is what later proves the published bytes are the ones that were
staged rather than something swapped in afterwards; that they are the ones that
were *tested* rests on the maintainer having tested this release's download.

The release goes in **this** repository; only the website's own data goes to
`taigikeyboard/taigikeyboard.github.io`:

| What | Where | Why there |
|---|---|---|
| The `.pkg` and its `.sha256` | assets on the GitHub release `desktop-<version>` **in this repository**, shared with the Windows installer | One desktop version is one release, beside the source it was built from: the tag names that commit, the notes are that commit's changelog. Releases lived on the website repository while this one was private and nothing served from it was anonymously reachable; it has been public since 2026-09-07. Release assets live outside git either way, so they cost no repository its size or bandwidth allowance. |
| `_data/macos_release.json` | committed site data in the website repository — written only by the announcement | The landing page's macOS download button reads it and links straight at the package, so its URL carries the version. Keeping it as data the release flow writes is what stops the page hard-coding a version, and what keeps the button off `/releases/latest` — that alias is repository-wide, and this repository's last release may be a Windows installer. |
| `appcast/macos.json` | **rendered** from that data by the site's own build, served at `https://taigikeyboard.tw/appcast/macos.json` | Every installed copy has that URL baked in (`UpdateChecker.publishedURL`) and expects `version` / `downloadPageURL` / `packageURL`, so the manifest stays a static file on the project's own domain rather than anything GitHub serves. Rendered rather than written because two files meant two commits, and two Pages runs seconds apart deploy their own trees: see *One published fact, one committed file* in `macos/updates/README.md` for the day the manifest sat a release behind. |

The pieces that are not macOS-specific — the draft, its tag identity, the
create-or-attach rule, the read-back, and the whole announcement — live in
`scripts/lib/desktop-release.sh`, `scripts/lib/release-site.sh` and
`scripts/announce-release.sh`, shared with Windows. `publish-release.sh` owns
what only a Mac can say about the package.

### What staging checks, in order

1. The shared preconditions, before any evidence is gathered about the package,
   so a mistake costs a second rather than a notarization wait: `gh` present and
   authenticated, a dotted-integer version, a clean tree, HEAD an ancestor of
   `origin/main`, and `changelog/desktop-v<version>.md` present **in that
   commit**. Not `main`'s tip — `main` may move between the two platforms'
   staging runs, and requiring the tip would strand whichever runs second.
2. The package is stapled, passes Gatekeeper, and — read out of its own
   `Distribution` — declares this bundle identifier at this build version.
   `--pkg` can point at any file, and the tag and manifest version both come
   from `Info.plist`, so a stale package would otherwise be staged under the
   current version's name.
3. Whatever already exists for this version names this commit. Two things can:
   a git tag (from a hand-push, or a release published earlier) is
   dereferenced and compared; a draft has no tag, only `targetCommitish`, which
   must be this exact commit SHA. A branch name there would tag whatever that
   branch points at on publish day, so it is refused. A release that is already
   published is refused outright — an asset added to it would be public
   immediately, which is the one thing this flow exists to prevent.
4. `gh release create --draft --target <commit>` with the whole
   `changelog/desktop-v<version>.md` as the notes (both platforms share the
   page, so both sections belong on it), or an upload into the existing draft.
   Never `--clobber`, and never over an asset already staged: an identical one
   is verified in place, a differing one stops the run.
5. The staged asset is downloaded back — authenticated, since a draft has no
   anonymous URL — and its SHA-256 compared to the local file's, so what the
   maintainer is about to test is provably what was built.

### What announcing checks, in order

1. The release is published, not a draft, and carries the tag.
2. For each platform's installer on it: the asset **and** its `.sha256` receipt
   are fetched **with no credentials at all** — `curl -q --netrc-file /dev/null`
   is what guarantees that; an authenticated check cannot tell a public URL from
   a private one, which is how the first version of this shipped pointing at a
   private repository. The downloaded bytes must hash to what the receipt says,
   which is what makes "what a user downloads" the same thing as "what was
   tested" rather than merely "what GitHub currently holds".
3. Both platforms' data files are written to the website in **one** commit, so
   the two cannot race each other's Pages deployment.
4. Each live appcast is polled until it serves this version **and** its package
   URL (and on Windows the digest). Both fields, not just the version, because a
   render that dropped `packageURL` still reads as a valid update and would
   quietly cost every install the in-app download. The poll is also the only
   thing that proves the site built what was committed, since the manifest is
   rendered rather than written.

Step 2 gates step 3 on purpose. The manifest is what every installed copy polls,
so announcing a version before its download is reachable points all of them at
a 404 — and the developer's own browser, being logged in, cannot see it happen.

A platform whose installer is not on the release is skipped with a note, and its
manifest is left where it was: one platform can lag, and announcing the one that
is ready beats making it wait.

`macos/updates/README.md` documents the manifest wire format.

After installing, the input method has to be added in System Settings → Keyboard
→ Input Sources. The input-source list is cached per login session, so a first
install may not appear until the user logs out and back in.

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
- **No auto-update.** Sparkle is not wired in, so users upgrade by downloading a
  new package.

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
| `onConclusionScript` re-login prompt | `MacishType/macos/Makefile:220` | the input-source cache note lives in the download page instead |
| `codesign --deep` when signing | `vChewing-macOS/BuildPKG.sh` | the bundle has no nested code; see above |
