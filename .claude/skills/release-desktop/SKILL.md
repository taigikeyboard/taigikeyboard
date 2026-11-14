---
name: release-desktop
description: Prepare a DESKTOP release (macOS + Windows + Linux + ChromeOS, one shared version) of the version already in the tree - upgrade check, rebuild, commit, and stage the installers on an unpublished draft; stops before publishing. Optional args: a patch platform (`macos` / `windows` / `linux` / `chromeos`) or a release base. Mobile train (iOS + Android) is `release-mobile`.
disable-model-invocation: true
---

# Release Desktop

The desktop train is macOS + Windows + Linux + ChromeOS, sharing one version, moved by
`make version-desktop YYYY.M.P` (CalVer), unrelated to the mobile number. The mobile train
has its own skill (`release-mobile`); a release never mixes the two.

Run from `main`. Takes no version: **`<target>` is whatever version the tree
already carries** — the maintainer sets it with `make version-desktop YYYY.M.P`
before invoking. Example: `/release-desktop`

Optional arguments, in any order:

- `<platform>` — `macos`, `windows`, `linux` or `chromeos`: a **patch release** of that
  platform alone. Example: `/release-desktop linux`
- `<base-ref>` — overrides the derived release base.
  Example: `/release-desktop desktop-3.6.7`

**Full release vs patch**: a full release (`YYYY.M.0`) ships every platform; a
patch (`YYYY.M.P`, P > 0) ships only the platform it fixes. Policy:
`docs/architecture/desktop-release.md` § Version numbers.

This skill takes a release as far as it can go without a person: it rebuilds,
commits, and stages **every** platform's installers on a draft release
nobody can reach (staging also dispatches the beta run for the unlisted /beta
page). It stops there. Publishing that draft is the maintainer's,
because it is the decision the draft exists to protect — and publishing
announces the release itself.

## 1. Validate context

- Require a clean worktree and `HEAD = main`.
- Run `git fetch --prune --tags --force` and `git pull --ff-only origin main`.

Derive `<target>` from the tree, never from an argument:

```bash
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' macos/App/Info.plist
python3 tools/release_notes.py check-versions --train desktop --version <target>
```

That check is what proves `windows/Cargo.toml`, `desktop/Cargo.toml`,
`linux/Cargo.toml`, `macos/Cargo.toml`, `web/chromeos/manifest.json` and both macOS plist keys carry the same number, so a
half-applied bump stops here instead of shipping desktop platforms on different
versions. When they disagree, run `make version-desktop <target>` — it writes
every file in one pass, or none, then refreshes each desktop `Cargo.lock`'s
own member versions (commit them with the bump).
Unlike the iOS `.pbxproj`, these files are not user-owned, so the skill may run
that itself.

Derive the release base unless one was passed:

```bash
git describe --tags --abbrev=0 --match 'desktop-*' HEAD
```

No `desktop-*` tag reachable from `HEAD` → stop and ask for `<base-ref>`.

Then, before any edit:

- Require `<target>` to match `YYYY.M.P`, and no published `desktop-<target>`
  release.
- Kind check: a `<platform>` argument with a `.0` target, or no `<platform>`
  with a non-`.0` target, breaks the numbering convention — name the mismatch
  and ask before continuing (the version is the maintainer's call).
- Report open PRs and ask before continuing if any exist.
- **State the derived version and range — `preparing desktop <target>, range
  <base>..HEAD, N commits` — and wait for the user to confirm.** Nothing is
  derived silently, because nothing was passed in.

**The desktop train only moves upward** — `set-versions` refuses a downgrade
(`docs/architecture/manual-release-notes.md` § Set the version).

`check-versions --train mobile` is not this skill's gate — an iOS/Android
version differing from `<target>` is expected, not a finding.

## 2. Upgrade check

Read the release range:

```bash
git log --first-parent <base-ref>..HEAD --oneline
git diff <base-ref>..HEAD --stat --name-status
```

Run the `upgrade-check` procedure for `<base-ref> → HEAD`:

- `BLOCKED` → stop before rebuild or edits.
- `CLEAN WITH BEHAVIOR CHANGES` → list each behavior change in the final report.
- `CLEAN` → continue.

Two desktop-specific upgrade checks on top of it:

- **User data.** The three desktops share the SQLite schemas under the
  per-user data directory (`%APPDATA%\TaigiKeyboard` on Windows,
  `~/.local/share/taigikeyboard` + `~/.config/taigikeyboard/settings.json` on
  Linux). A schema change ships to all at once; say what an existing install
  sees on first launch.
- **Installed-file set.** A dictionary, font, or Windows App Runtime file that
  moved or was removed changes what the installer stages. `windows/scripts/release-app.sh`
  stages `Dictionaries\` and `Fonts\` from the repository root, and its
  preflight fails on an empty one. The Linux packages (`.deb`, `.rpm`,
  Arch) take their file set from `make -C linux install`
  (`docs/architecture/linux-release.md` § The artifacts),
  and `linux-build.yml` fails when an expected path is missing from it.

## 3. Rebuild release artifacts

**The skill runs these itself** — the artifact shipped must be built from the
commit being released.

**Always**, not only when the range touched them — a clean tree proves nothing
about the gitignored engine binaries (`docs/architecture/build-artifacts.md`
§ A release rebuilds first).

```bash
make i18n
make build
```

`make dict` is the exception: run it only when the range touched `dictionary/`
sources. Its outputs are committed, and rebuilding them produces byte-different
`association.bin` / `dictionary.bin` on every run, so an unconditional pass
would put noise in the release commit.

```bash
RELEASE_VERSION=v<target> make dict   # only if dictionary/ sources moved; takes vX.Y.Z
```

Commit whatever the rebuild changed before staging: staging refuses a dirty
tree.

## 4. Release notes — nothing to write

Staging fills the draft's body with GitHub's generated notes: every pull
request merged since the previous published `desktop-*` release, with its
author, plus new contributors and a compare link
(`tools/release/lib/desktop-release.sh` `_write_release_notes`;
`docs/architecture/release-automation-roadmap.md` D2). Do not create
`changelog/desktop-v<target>.md` or add to the desktop list in `CHANGELOG.md` —
that list ends at v3.9.0. There are **no store notes on this train**: do not
create `changelog/store/<target>/`, do not run `release_notes.py check` or
`print`.

## 5. Validate

```bash
python3 tools/release_notes.py check-versions --train desktop --version <target>
python3 tools/release_notes_test.py
git diff --check
```

## 6. Commit

Only when § 1 or § 3 changed files (a version bump, regenerated artifacts):

```bash
git add -A
git commit -m "desktop v<target>: release prep"
git push origin main
```

Report the commit SHA, the release range and any behavior changes from § 2,
then go straight to staging.

## 7. Stage every platform

```bash
make desktop-release
```

It creates the draft and dispatches the hosted macOS (signed + notarized on the
runner), Windows, Linux and ChromeOS builds from the same commit onto the **draft** `desktop-<target>` (what each
step does and checks: `docs/architecture/desktop-release.md` § The flow,
§ What staging checks). A full release is every platform or none; an existing draft
for this version is deleted first, so if a half fails, fix that and run the
whole thing again.

For a **patch**, stage only its platform:

```bash
make desktop-patch PLATFORM=<platform>
```

Same script, same clean draft from one commit, with one platform's installers
(Linux: `.deb`, `.rpm` and Arch together).

Report the draft URL.

## Hand off — the one manual step

**Test what was staged, then publish it** — on the draft's own page, which step
7 printed. A draft is visible in the web UI to anyone who can write this
repository: the installers are download links there, and **Publish release** is a
button on the same page. This is the decision the draft exists to protect, so the
skill never presses it.

Install each (the `.deb` on the Linux VM, S74; the ChromeOS zip unpacked in
the ChromeOS VM, `docs/architecture/chromeos-roadmap.md` § Spike result), run the manual checks
this release touches, then publish.
(The same two steps from a terminal, if that is closer to hand:
`gh release download desktop-<target> --repo taigikeyboard/taigikeyboard --dir ~/Downloads`
and `gh release edit desktop-<target> --repo taigikeyboard/taigikeyboard --draft=false`.)

Publishing creates the tag and runs the announcement
(`docs/architecture/desktop-release.md` § The flow, § What announcing checks).

The website's Linux button stays hidden until `enable_linux_download` is `true`
in the website's `_config.yml` — the maintainer's switch, like the publish. It
hides the button, not the packages, which are public once published.

**After a release carrying the ChromeOS zip is published**, remind the
maintainer to upload it to the Chrome Web Store (the announcement prints the
same reminder; `docs/architecture/chromeos-release.md` § Upload).

**After a release carrying a macOS `.pkg` is published**, the announcement
bumps and audits the Homebrew cask (`docs/architecture/desktop-release.md`
§ The flow). After a local `make desktop-announce`, bump it only when asked:
`bash tools/release/bump-cask.sh <macos_version> <macos_sha256>`.

## Guardrails

- Never write a desktop changelog file — the notes are generated (§ 4).
- Never touch `changelog/mobile-v<version>.md` or `changelog/store/**` — that is
  `release-mobile`'s surface, and desktop-only work must never enter a store note.
- Never bump the Homebrew cask before the release is published.
- Never publish or un-draft a release, and never push a tag by hand: publishing
  is the manual step this whole flow is shaped around, and it is what creates the
  tag.
- Never run `make desktop-announce` or `announce-release.sh` — the publish runs
  the announcement.
- Never request, store, or use signing certificates, notarization credentials,
  or thumbprints.
- Never pass `--allow-downgrade`, `--allow-dirty`, or `--force` to any release
  tool.
- Surface the first actionable rebuild or validation failure and stop before
  commit.
