---
name: release-desktop
description: Stage a DESKTOP hotfix — a patch release (`YYYY.M.P`, P > 0) of ONE desktop platform (`macos` / `windows` / `linux` / `chromeos`) at the version already in the tree - upgrade check, rebuild, commit, and stage that platform's installers on an unpublished draft; stops before publishing. Required arg: the platform; optional: a release base. The monthly full release is the automatic train (`desktop-train.yml` / `desktop-publish.yml`), not this skill. Mobile train (iOS + Android) is `release-mobile`.
disable-model-invocation: true
---

# Release Desktop — hotfix patch

The desktop train is macOS + Windows + Linux + ChromeOS, sharing one CalVer
version, unrelated to the mobile number. The **monthly full release
(`YYYY.M.0`) runs itself**: `desktop-train.yml` cuts and stages on the 1st,
`desktop-publish.yml` publishes on the 5th
(`docs/architecture/desktop-release.md` § The flow). This skill is only the
**hotfix**: a patch of one platform after a release is published
(`docs/architecture/desktop-release.md` § Hold, hotfix, re-stage).

Run from `main`. Takes no version: **`<target>` is whatever version the tree
already carries** — the maintainer sets it with `make version-desktop YYYY.M.P`
before invoking.

Arguments, in any order:

- `<platform>` (required) — `macos`, `windows`, `linux` or `chromeos`: the one
  platform the patch ships. Example: `/release-desktop linux`
- `<base-ref>` — overrides the derived release base.
  Example: `/release-desktop linux desktop-2026.12.0`

Not this skill — say so and stop:

| Situation | What to do instead |
|---|---|
| No `<platform>`, or a `.0` target (a full release) | The train: `gh workflow run desktop-train.yml` (or wait for the 1st) |
| The month's `desktop-YYYY.M.0` draft is staged and unpublished (between the 1st and the 5th) | Fix on `main`, then `gh workflow run desktop-train.yml` re-stages the draft from `main`'s tip |
| Hold the train | `gh variable set RELEASE_HOLD --body "<reason>"` |

This skill takes the patch as far as it can go without a person: it rebuilds,
commits, and stages the platform's installers on a draft release nobody can
reach (staging also dispatches the beta run for the unlisted /beta page). It
stops there. The publish schedule only picks up the month's `.0` draft, so a
patch is published by hand — that is the maintainer's, because it is the
decision the draft exists to protect, and publishing announces the release.

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
half-applied bump stops here. When they disagree, run
`make version-desktop <target>` — it writes every file in one pass, or none,
then refreshes each desktop `Cargo.lock`'s own member versions (commit them
with the bump). Unlike the iOS `.pbxproj`, these files are not user-owned, so
the skill may run that itself.

Derive the release base unless one was passed:

```bash
git describe --tags --abbrev=0 --match 'desktop-*' HEAD
```

No `desktop-*` tag reachable from `HEAD` → stop and ask for `<base-ref>`.

Then, before any edit:

- Require `<platform>`, and `<target>` matching `YYYY.M.P` with P > 0 (a patch
  before the first train is `2026.11.P`). Otherwise stop with the table above.
- Require no published `desktop-<target>` release.
- Require no unpublished draft of the month's full release:
  `gh release view desktop-<YYYY.M>.0 --json isDraft` reporting `true` → stop
  with the table above.
- Report open PRs and ask before continuing if any exist.
- **State the derived version and range — `preparing desktop <target>
  <platform> patch, range <base>..HEAD, N commits` — and wait for the user to
  confirm.** Nothing is derived silently, because nothing was passed in.

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

Two desktop-specific upgrade checks on top of it, for the patched platform:

- **User data.** The three desktops share the SQLite schemas under the
  per-user data directory (`%APPDATA%\TaigiKeyboard` on Windows,
  `~/.local/share/taigikeyboard` + `~/.config/taigikeyboard/settings.json` on
  Linux). A schema change in a patch reaches only the patched platform until
  the next full release; say what an existing install sees on first launch.
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
request merged since the previous published `desktop-*` release
(`tools/release/lib/desktop-release.sh` `_write_release_notes`;
`docs/architecture/release-automation-roadmap.md` D2). Do not create
`changelog/desktop-v<target>.md` or add to the desktop list in `CHANGELOG.md` —
that list ends at v3.9.0. There are no store notes on this train.

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

## 7. Stage the platform

```bash
make desktop-patch PLATFORM=<platform>
```

It creates a clean draft `desktop-<target>` on this commit and dispatches that
platform's hosted build onto it (Linux: `.deb`, `.rpm` and Arch together;
macOS: signed + notarized on the runner, waiting for the `release`
environment approval). What each step checks:
`docs/architecture/desktop-release.md` § The flow, § What staging checks. An
existing draft for this version is deleted first, so if the build fails, fix
that and run it again.

Report the draft URL.

## Hand off — the one manual step

**Test what was staged, then publish it** — on the draft's own page, which step
7 printed. A draft is visible in the web UI to anyone who can write this
repository: the installers are download links there, and **Publish release** is a
button on the same page. The skill never presses it.

Install it (the `.deb` on the Linux VM, S74; the ChromeOS zip unpacked in the
ChromeOS VM, `docs/architecture/chromeos-roadmap.md` § Spike result), run the
manual checks the fix touches, then publish.
(The same two steps from a terminal:
`gh release download desktop-<target> --repo taigikeyboard/taigikeyboard --dir ~/Downloads`
and `gh release edit desktop-<target> --repo taigikeyboard/taigikeyboard --draft=false`.)

Publishing creates the tag and runs the announcement, which writes only the
patched platform's `_data` / appcast
(`docs/architecture/desktop-release.md` § The flow, § What announcing checks).

**A ChromeOS patch**: once published, remind the maintainer to upload the zip
to the Chrome Web Store (the announcement prints the same reminder;
`docs/architecture/chromeos-release.md` § Upload).

**A macOS patch**: the announcement bumps and audits the Homebrew cask
(`docs/architecture/desktop-release.md` § The flow).

## Guardrails

- Never stage a full release (`make desktop-release`) — that is the train's.
- Never write a desktop changelog file — the notes are generated (§ 4).
- Never touch `changelog/mobile-v<version>.md` or `changelog/store/**` — that is
  `release-mobile`'s surface.
- Never set or clear `RELEASE_HOLD`, and never dispatch `desktop-train.yml` or
  `desktop-publish.yml` — the maintainer's calls.
- Never bump the Homebrew cask by hand.
- Never publish or un-draft a release, and never push a tag by hand.
- Never run `make desktop-announce` or `announce-release.sh` — the publish runs
  the announcement.
- Never request, store, or use signing certificates, notarization credentials,
  or thumbprints.
- Never pass `--allow-downgrade`, `--allow-dirty`, or `--force` to any release
  tool.
- Surface the first actionable rebuild or validation failure and stop before
  commit.
