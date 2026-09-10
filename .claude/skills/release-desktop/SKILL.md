---
name: release-desktop
description: Prepare a DESKTOP release (macOS + Windows, one shared version) on main - rebuild generated artifacts, set the desktop version, write the `### macOS` and `### Windows` sections of `changelog/desktop-v<version>.md`, link it from CHANGELOG.md, validate, commit, and push. Use when preparing a macOS package or a Windows installer for publication. Never tags, builds a package, signs, notarizes, publishes, or runs `make macos-release` / `make windows-release`. Desktop train only; the mobile train (iOS + Android) is `release-mobile`.
---

# Release Desktop

The desktop train is macOS + Windows, sharing one version, moved by
`make version-desktop x.y.z`, unrelated to the mobile number. The mobile train
has its own skill (`release-mobile`); a release never mixes the two.

Run from `main`. Require `<base-ref> <target>`, target in semantic-version form.
Example: `/release-desktop desktop-3.6.7 3.6.8`

`<base-ref>` is the point the previous desktop release shipped from — the
`desktop-<version>` tag when one exists, otherwise the commit that bumped the
previous desktop version (`git log --oneline -S'CFBundleShortVersionString' -- macos/App/Info.plist`
finds it). Desktop releases before 2026-09 were never tagged in this repository;
their tags (`macos-v…` / `windows-v…`) live on the website repository.

This skill prepares the repository. It never builds, signs, notarizes, packages,
publishes, or tags — every one of those is the maintainer's own command, on the
right machine, and they are listed in § Hand off.

## 1. Validate context

- Require a clean worktree and `HEAD = main`.
- Run `git fetch --prune --tags --force` and `git pull --ff-only origin main`.
- Require `<base-ref>` to resolve and be an ancestor of `HEAD`.
- Require `<target>` to match `MAJOR.MINOR.PATCH`.
- Run `python3 tools/release_notes.py check-versions --train desktop --version <target>`.
- Report open PRs and ask before continuing if any exist.
- Use `<base-ref>..HEAD` as the release range. State it before edits.

When `check-versions` fails because the tree still carries the previous desktop
version, run `make version-desktop <target>` — it writes both macOS plist keys
and `windows/Cargo.toml` in one pass, or neither. Unlike the iOS `.pbxproj`,
these two files are not user-owned, so this skill may run that command itself.

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
  (its own shortcut spelling: `⌃⌘H` on macOS, `Ctrl+Alt+H` on Windows). A
  publish script extracts one section as that platform's whole release body, so
  a fact mentioned only in the other section reaches nobody.
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

Shape of `changelog/desktop-v<target>.md` — the publish scripts depend on it:

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

- `### macOS` and `### Windows` are matched literally by
  `macos/scripts/publish-release.sh` and `windows/scripts/publish-release.sh`.
  A missing section does not fail the publish — it silently degrades to a
  one-line `TaigiKeyboard for <platform> <version>` release body. Write both.
- English prose; Taigi terms, UI labels and examples keep 漢字 / TL / POJ / TPS.
- The section is the GitHub release body users read: concrete user-visible
  behavior, with the PR number in `(#NNN)`. No refactors, tests, tooling, or
  dependency bumps unless a user feels them.
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

Everything below is the maintainer's, on the machine named. This skill runs
none of it.

**1. macOS package — on the Mac, from a clean tree:**

```bash
make macos-release
```

Builds, signs with both Developer ID certificates, notarizes, staples, packages,
then `macos/scripts/publish-release.sh` uploads the `.pkg` to the
`taigikeyboard/taigikeyboard.github.io` release tagged `macos-v<target>` and
points `_data/macos_release.json` at it. The site renders
`appcast/macos.json` from that file — that manifest is what installed copies
check. Needs the notarization credential (`docs/architecture/macos-release.md`).

**2. Windows installer — on the Windows box (`ssh win`), from Git Bash, clean tree:**

```bash
make windows-release RELEASE_FLAGS=--skip-sign
```

There is no Authenticode certificate yet, so `--skip-sign` is the release
channel, stated explicitly so nothing publishes unsigned by accident. Builds,
stages, packages with Inno Setup, then `windows/scripts/publish-release.sh
--allow-unsigned` uploads to the `windows-v<target>` release on the website
repository and writes `_data/windows_release.json`; the published manifest
carries the installer's SHA-256, which is what the in-app updater verifies.
Before this: `make windows-check` on the Mac is the host-side gate.

**3. Tag, if the maintainer wants one:** `desktop-<target>` on this repository.
That tag is also the only trigger of `.github/workflows/windows-build.yml`
(plus manual dispatch), which rebuilds the installer on a GitHub-hosted runner
for SignPath provenance. It does not publish. Tagging is user-gated — never
create, move, or push it.

## Guardrails

- Never edit another version's changelog.
- Never touch `changelog/v<version>.md` or `changelog/store/**` — that is
  `release-mobile`'s surface, and desktop-only work must never enter a store note.
- Never run `make macos-release`, `make windows-release`, either
  `release-app.sh`, or either `publish-release.sh`.
- Never create, move, or push a tag; never create a GitHub release.
- Never request, store, or use signing certificates, notarization credentials,
  or thumbprints.
- Never pass `--allow-downgrade`, `--allow-dirty`, or `--force` to any release
  tool.
- Surface the first actionable rebuild or validation failure and stop before
  commit.
