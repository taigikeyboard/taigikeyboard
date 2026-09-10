---
name: release-desktop
description: Prepare a DESKTOP release (macOS + Windows, one shared version) on main - rebuild generated artifacts, set the desktop version, write the `### macOS` and `### Windows` sections of `changelog/desktop-v<version>.md`, link it from CHANGELOG.md, validate, commit, and push. Use when preparing a macOS package or a Windows installer for publication. Never tags, builds a package, signs, notarizes, publishes, or runs `make macos-release` / `make windows-release`. Takes no version argument - it releases the version already in the tree (set beforehand with `make version-desktop x.y.z`); an optional argument only overrides the release base. Desktop train only; the mobile train (iOS + Android) is `release-mobile`.
---

# Release Desktop

The desktop train is macOS + Windows, sharing one version, moved by
`make version-desktop x.y.z`, unrelated to the mobile number. The mobile train
has its own skill (`release-mobile`); a release never mixes the two.

Run from `main`. Takes no version: **`<target>` is whatever version the tree
already carries** — the maintainer sets it with `make version-desktop x.y.z`
before invoking. Example: `/release-desktop`

One optional argument, `<base-ref>`, overrides the derived release base.
Example: `/release-desktop desktop-3.6.7`

This skill prepares the repository. It never builds, signs, notarizes, packages,
publishes, or tags — every one of those is the maintainer's own command, on the
right machine, and they are listed in § Hand off.

## 1. Validate context

- Require a clean worktree and `HEAD = main`.
- Run `git fetch --prune --tags --force` and `git pull --ff-only origin main`.

Derive `<target>` from the tree, never from an argument:

```bash
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' macos/App/Info.plist
python3 tools/release_notes.py check-versions --train desktop --version <target>
```

That check is what proves `windows/Cargo.toml` and both macOS plist keys carry
the same number, so a half-applied bump stops here instead of shipping two
desktop platforms on different versions. When they disagree, run
`make version-desktop <target>` — it writes all three in one pass, or none.
Unlike the iOS `.pbxproj`, these files are not user-owned, so the skill may run
that itself.

Derive the release base unless one was passed:

```bash
git describe --tags --abbrev=0 --match 'desktop-*' HEAD
```

Desktop releases cut before 2026-09-09 were tagged on the website repository
(`macos-v…` / `windows-v…`), not here. With no `desktop-*` tag, fall back to
the commit that set the current version:

```bash
git log --format='%h %ad %s' --date=short -S'<target>' -- macos/App/Info.plist | tail -1
```

That bump is the proxy for where the previous release shipped from, and it is a
proxy — anything merged between that release and the bump falls outside the
range. Say so when using it, and prefer an explicit `<base-ref>`.

Then, before any edit:

- Require `<target>` to match `MAJOR.MINOR.PATCH`, and
  `changelog/desktop-v<target>.md` to be absent or not yet published.
- Report open PRs and ask before continuing if any exist.
- **State the derived version and range — `preparing desktop <target>, range
  <base>..HEAD, N commits` — and wait for the user to confirm.** Nothing is
  derived silently, because nothing was passed in.

**The desktop train only moves upward.** `CFBundleVersion` derives as
`MAJOR*10000 + MINOR*100 + PATCH` and is what the macOS Installer compares
between packages; `set-versions` refuses a downgrade. Never pass
`--allow-downgrade`.

`check-versions --train mobile` is not this skill's gate — an iOS/Android
version differing from `<target>` is expected, not a finding.

## 2. Analyze the release

Read the complete release range:

```bash
git log <base-ref>..HEAD --oneline
git diff <base-ref>..HEAD --stat --name-status
git log <base-ref>..HEAD --format="%s%n%b%n---"
```

Trace user-visible behavior by platform. Do not infer release scope from commit
subjects alone.

Sort every user-visible change before writing anything:

- **macOS-only** → the `### macOS` section.
- **Windows-only** → the `### Windows` section.
- **Shared** (engine, dictionary, a behavior landing on both desktop platforms)
  → describe it in **both** sections, each in that platform's own terms
  (its own shortcut spelling: `⌃⌘H` on macOS, `Ctrl+Alt+H` on Windows). Both
  platforms share one release page carrying the whole file, so a reader on
  either platform should find their own wording of the change under their own
  heading rather than having to read the other platform's section for it.
- **iOS-only / Android-only** → NOT this release. Mobile work belongs to
  `changelog/v<version>.md` and the store notes, written by `release-mobile`.

Run the `upgrade-check` procedure for `<base-ref> → HEAD`:

- `BLOCKED` → stop before rebuild or edits.
- `CLEAN WITH BEHAVIOR CHANGES` → record each behavior change in the affected
  platform's section.
- `CLEAN` → continue.

Two desktop-specific upgrade checks on top of it:

- **User data.** macOS and Windows share the SQLite schemas under the per-user
  data directory (`%APPDATA%\TaigiKeyboard` on Windows). A schema change ships
  to both at once; say what an existing install sees on first launch.
- **Installed-file set.** A dictionary, font, or Windows App Runtime file that
  moved or was removed changes what the installer stages. `windows/scripts/release-app.sh`
  stages `Dictionaries\` and `Fonts\` from the repository root, and its
  preflight fails on an empty one.

## 3. Rebuild release artifacts

Only when the range touched `engine/` or `dictionary/` (`CLAUDE.md` §
stale-artifact gate). The release scripts deliberately do **not** rebuild them:
a release from a clean tree ships exactly what is committed, so a stale
committed artifact ships stale.

```bash
RELEASE_VERSION=<target> make dict   # only if dictionary/ moved
make i18n
make build
```

## 4. Update release content

| File | Purpose |
| --- | --- |
| `changelog/desktop-v<target>.md` | The desktop record: a lead paragraph, then `### macOS` and `### Windows` |
| `CHANGELOG.md` | Link to it, at the top of the `## Desktop — macOS + Windows` list (newest first) |

Shape of `changelog/desktop-v<target>.md` — this whole file becomes the release
body, so it is what users read on the release page:

```markdown
# desktop v<target>

<One paragraph: what this release is, in a sentence or three.>

### macOS

#### <Input | Candidates | Settings | Appearance | Updates | Install>

- **<Short claim.>** <What a user sees, and why it changed.> (#PR)

### Windows

#### <same shape>
```

Rules:

- The file must be **committed** before either platform publishes: the release
  body is read out of the tagged commit, not the working tree, and a version
  with no changelog in that commit fails the publish outright.
- English prose; Taigi terms, UI labels and examples keep 漢字 / TL / POJ / TPS.
- This is the GitHub release body users read: concrete user-visible behavior,
  with the PR number in `(#NNN)`. No refactors, tests, tooling, or dependency
  bumps unless a user feels them.
- A subsection per surface, not one flat list — these bodies run long and the
  headings are what make them readable.
- Idempotent: re-running on an existing target file merges by topic. Refine the
  line for a behavior; never add a second line for it.
- There are **no store notes on this train.** Do not create
  `changelog/store/<target>/`, do not run `release_notes.py sync`, `check`, or
  `print` — those validate the mobile surface only and will fail or mislead here.

## 5. Validate

```bash
python3 tools/release_notes.py check-versions --train desktop --version <target>
python3 tools/release_notes_test.py
git diff --check
```

Then read both sections back against the release range: every PR in the range
that changed user-visible desktop behavior appears in at least one section, and
every claim in a section is traceable to a commit in the range.

Deterministic validation does not replace the user's factual review.

## 6. Commit and hand off

```bash
git add -A
git commit -m "desktop v<target>: release prep + changelog"
git push origin main
```

Report the commit SHA, the release range, and both rendered sections.

## Hand off

Everything below is the maintainer's, on the machine named. This skill runs none
of it. The release is a **draft** until step 4: no tag, no public download,
nothing a user can reach. Full procedure and rationale:
`docs/architecture/desktop-release.md`.

**1. Stage the macOS package — on the Mac, from a clean tree:**

```bash
make macos-release
```

Builds, signs with both Developer ID certificates, notarizes, staples, packages,
then `macos/scripts/publish-release.sh` puts the `.pkg` (and a `.sha256`
receipt) on the **draft** `desktop-<target>` release in this repository —
creating it, recording the commit the checkout is at as what publishing will
tag, with the whole `changelog/desktop-v<target>.md` as its body. Needs the
notarization credential (`docs/architecture/macos-release.md`).

**2. Stage the Windows installer — on the Windows box (`ssh win`), Git Bash, clean tree:**

```bash
make windows-release RELEASE_FLAGS=--skip-sign
```

There is no Authenticode certificate yet, so `--skip-sign` is the release
channel, stated explicitly so nothing ships unsigned by accident. Builds,
stages, packages with Inno Setup, then attaches the installer and its receipt to
the **same** draft (creating it if Windows goes first). Before this:
`make windows-check` on the Mac is the host-side gate.

Whichever platform stages first creates the draft; the second moves the draft's
target (and any tag) onto its own commit. A release that has already been
**published** is the exception — it stops rather than adding an asset that would
be public with no test.

**3. Test what was staged — the whole point of the draft:**

```bash
gh release download desktop-<target> --repo taigikeyboard/taigikeyboard --dir ~/Downloads
```

Install both, run the dogfood checklist items this release touches.

**4. Publish, when both pass — the last manual step:**

```bash
gh release edit desktop-<target> --repo taigikeyboard/taigikeyboard --draft=false
```

This creates the tag and fires two workflows:
`.github/workflows/announce-release.yml`, which proves both downloads are
anonymously reachable, writes both `_data/*_release.json` to the website in one
commit and waits for the live appcasts (a platform whose installer is absent is
skipped with a note); and `.github/workflows/windows-build.yml`, a GitHub-hosted
rebuild for SignPath provenance that publishes nothing.

`make desktop-announce` runs the same announcement by hand — for a re-run after
a failed job, or when its token has expired.

## Guardrails

- Never edit another version's changelog.
- Never touch `changelog/v<version>.md` or `changelog/store/**` — that is
  `release-mobile`'s surface, and desktop-only work must never enter a store note.
- Never run `make macos-release`, `make windows-release`, `make desktop-announce`,
  either `release-app.sh`, either `publish-release.sh`, or `announce-release.sh`.
  The hand-off section tells the maintainer to run them; the skill never does.
- Never create, move, or push a tag; never create, publish, or un-draft a GitHub
  release.
- Never request, store, or use signing certificates, notarization credentials,
  or thumbprints.
- Never pass `--allow-downgrade`, `--allow-dirty`, or `--force` to any release
  tool.
- Surface the first actionable rebuild or validation failure and stop before
  commit.
