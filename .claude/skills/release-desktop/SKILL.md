---
name: release-desktop
description: Prepare a DESKTOP release (macOS + Windows + Linux + ChromeOS, one shared version) of the version already in the tree - changelog, rebuild, commit, and stage the installers on an unpublished draft; stops before publishing. Optional args: a patch platform (`macos` / `windows` / `linux` / `chromeos`) or a release base. Mobile train (iOS + Android) is `release-mobile`.
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
assembles the changelog from fragments, commits, and stages **every** platform's installers on a draft release
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

- Require `<target>` to match `YYYY.M.P`, and
  `changelog/desktop-v<target>.md` to be absent or not yet published.
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

## 2. Analyze the release

Read the complete release range:

```bash
git log <base-ref>..HEAD --oneline
git diff <base-ref>..HEAD --stat --name-status
git log <base-ref>..HEAD --format="%s%n%b%n---"
```

For a patch, the changelog covers only what reaches that platform: its own tree
(`macos/`, `windows/`, `linux/`, `web/chromeos/`) plus the shared `desktop/`, `engine/` and
`dictionary/` changes its build picks up.

Trace user-visible behavior by platform. Do not infer release scope from commit
subjects alone.

The release notes are assembled from `changelog/unreleased/*.md` fragments
(§ 4), one per pull request. This step checks that coverage: every
user-visible change in the range has a fragment, under the right platforms and
section. A missing one (a PR merged with `no-changelog` that users do see, a
direct commit) → write the fragment now, ending its bullet with the PR's
`(#NNN)`, and commit it before § 4. Format:
`changelog/unreleased/README.md`. Sort by these rules:

- **macOS-only** → the `### macOS` section.
- **Windows-only** → the `### Windows` section.
- **Linux-only** → the `### Linux` section.
- **ChromeOS-only** → the `### ChromeOS` section (it shares `web/src/` and the
  wasm engine with the web version, which has no release of its own).
- **Shared** (engine, dictionary, a behavior landing on several desktop
  platforms) → describe it in **each affected platform's** section, in that
  platform's own terms (its own shortcut spelling: `⌃⌘H` on macOS,
  `Ctrl+Alt+H` on Windows and Linux). A change that touches `desktop/` or
  `windows/` shared code but not Linux's shell path (or the reverse) goes only
  where a user can see it. All platforms share one release page carrying the
  whole file, so a reader on any platform should find their own wording of the
  change under their own heading. A Linux section with nothing new says so in
  one line rather than being left out.
- **iOS-only / Android-only** → NOT this release. Mobile work belongs to
  `changelog/mobile-v<version>.md` and the store notes, written by `release-mobile`.

Run the `upgrade-check` procedure for `<base-ref> → HEAD`:

- `BLOCKED` → stop before rebuild or edits.
- `CLEAN WITH BEHAVIOR CHANGES` → record each behavior change in the affected
  platform's section.
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

## 4. Update release content

Assemble the notes from the fragments — never write the file by hand:

```bash
python3 tools/release_notes.py assemble --train desktop --version <target>                        # YYYY.M.0
python3 tools/release_notes.py assemble --train desktop --version <target> --platform <platform>  # patch
```

| File | What `assemble` does |
| --- | --- |
| `changelog/desktop-v<target>.md` | Writes the desktop record: `# desktop v<target>`, a one-line lead naming the platforms with changes, then `### macOS`, `### Windows`, `### Linux`, `### ChromeOS`, each with its `####` sections (`No changes.` when none) |
| `CHANGELOG.md` | Links it at the top of the `## Desktop — macOS + Windows + Linux + ChromeOS` list (patch: `— <Platform>-only patch`) |
| `changelog/unreleased/*.md` | Deletes the fragments it used. A patch takes only fragments naming its platform; one naming other platforms too loses the patched platform and stays for the next full release |

It appends each entry's `(#NNN)` from the merge commit that added the
fragment (a bullet that already ends in `(#NNN)` keeps it), so it needs full
history and a committed fragment; it warns on any entry left without a number.
It refuses when the target file already exists or there is no fragment for the
scope.

Rules:

- A patch's file has the lead line and ONLY its platform's `###` section; the
  release page shows what this version ships.
- The file must be **committed** before staging: the release body is read out
  of the staged commit (the one the tag will name), not the working tree, and a
  commit with no changelog fails staging outright.
- Fragment wording rules (they become the release body users read): English
  only — UI labels by their i18n `en` value (look up `"hanji": "<label>"` in
  `i18n/*.json`), quotes translated; CJK / TL / POJ / TPS only for Taigi
  example words and readings. Concrete user-visible behavior; no refactors,
  tests, tooling, or dependency bumps unless a user feels them.
- Re-running after staging failed: `changelog/desktop-v<target>.md` is already
  committed, so skip `assemble` and go straight to § 7. A fragment merged after
  the assemble waits for the next release.
- There are **no store notes on this train.** Do not create
  `changelog/store/<target>/`, do not run `release_notes.py check` or
  `print` — those validate the mobile surface only and will fail or mislead here.

## 5. Validate

```bash
python3 tools/release_notes.py check-versions --train desktop --version <target>
python3 tools/release_notes_test.py
git diff --check
```

Then read every section back against the release range: every PR in the range
that changed user-visible desktop behavior appears in at least one section, and
every claim in a section is traceable to a commit in the range.

## 6. Commit

```bash
git add -A
git commit -m "desktop v<target>: release prep + changelog"
git push origin main
```

Report the commit SHA, the release range, and every rendered section, then go
straight to staging: the maintainer reads the changelog on the draft page before
publishing, since deterministic validation cannot tell whether a sentence
describes the behavior that shipped.

## 7. Stage every platform

```bash
make desktop-release
```

It builds the macOS `.pkg` here and dispatches the hosted Windows, Linux and
ChromeOS builds from the same commit onto the **draft** `desktop-<target>` (what each
step does and checks: `docs/architecture/desktop-release.md` § The flow,
§ What staging checks). A full release is every platform or none; an existing draft
for this version is deleted first, so if a half fails or `main` moved while it
was waited on, fix that and run the whole thing again.

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

**After a release carrying a macOS `.pkg` is published**, remind the maintainer
to bump the Homebrew cask (or do it when asked) — steps in
`docs/architecture/desktop-release.md` § The flow; also run
`brew style taigikeyboard/tap` before pushing.

## Guardrails

- Never edit another version's changelog.
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
