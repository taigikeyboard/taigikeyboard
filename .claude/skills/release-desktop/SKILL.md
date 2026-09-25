---
name: release-desktop
description: Prepare a DESKTOP release (macOS + Windows + Linux, one shared version) of the version already in the tree - changelog, rebuild, commit, and stage the installers on an unpublished draft; stops before publishing. Optional args: a patch platform (`macos` / `windows` / `linux`) or a release base. Mobile train (iOS + Android) is `release-mobile`.
---

# Release Desktop

The desktop train is macOS + Windows + Linux, sharing one version, moved by
`make version-desktop x.y.z`, unrelated to the mobile number. The mobile train
has its own skill (`release-mobile`); a release never mixes the two.

Run from `main`. Takes no version: **`<target>` is whatever version the tree
already carries** — the maintainer sets it with `make version-desktop x.y.z`
before invoking. Example: `/release-desktop`

Optional arguments, in any order:

- `<platform>` — `macos`, `windows` or `linux`: a **patch release** of that
  platform alone. Example: `/release-desktop linux`
- `<base-ref>` — overrides the derived release base.
  Example: `/release-desktop desktop-3.6.7`

**Full release vs patch** (USER 2026-09-24): a full release bumps the minor
number and ships every platform (3.7.0, 3.8.0); a patch bumps the third number
and ships only the platform it fixes (3.7.1 macOS, 3.7.2 Windows — one shared
counter, one version per patch). Rationale and the announce side:
`docs/architecture/desktop-release.md` § Version numbers.

This skill takes a release as far as it can go without a person: it rebuilds,
writes the changelog, commits, and stages **all three** installers on a draft release
nobody can reach. It stops there. Publishing that draft is the maintainer's,
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
`linux/Cargo.toml` and both macOS plist keys carry the same number, so a
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

- Require `<target>` to match `MAJOR.MINOR.PATCH`, and
  `changelog/desktop-v<target>.md` to be absent or not yet published.
- Kind check: a `<platform>` argument with a `.0` target, or no `<platform>`
  with a non-`.0` target, breaks the numbering convention — name the mismatch
  and ask before continuing (the version is the maintainer's call).
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

For a patch, the changelog covers only what reaches that platform: its own tree
(`macos/`, `windows/`, `linux/`) plus the shared `desktop/`, `engine/` and
`dictionary/` changes its build picks up.

Trace user-visible behavior by platform. Do not infer release scope from commit
subjects alone.

Sort every user-visible change before writing anything:

- **macOS-only** → the `### macOS` section.
- **Windows-only** → the `### Windows` section.
- **Linux-only** → the `### Linux` section.
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
  preflight fails on an empty one. The `.deb` takes its file set from
  `make -C linux install` (`docs/architecture/linux-release.md` § The artifact),
  and `linux-build.yml` fails when an expected path is missing from it.

## 3. Rebuild release artifacts

**The skill runs these itself** — the artifact shipped must be built from the
commit being released.

**Always**, not only when the range touched them. The engine binaries a platform
links are generated and gitignored (`macos/RustEngine/RustTaigi.xcframework/`,
`android/app/src/main/jniLibs/**`), so nothing in the tree says which commit the
local copy was built from — a clean tree proves nothing about them, and a
machine that last built on another branch would ship that. Rebuilding costs
seconds against a warm target directory.

```bash
make i18n
make build
```

`make dict` is the exception: run it only when the range touched `dictionary/`
sources. Its outputs are committed, and rebuilding them produces byte-different
`association.bin` / `dictionary.bin` on every run, so an unconditional pass
would put noise in the release commit.

```bash
RELEASE_VERSION=<target> make dict   # only if dictionary/ sources moved
```

Commit whatever the rebuild changed before staging: staging refuses a dirty
tree.

## 4. Update release content

| File | Purpose |
| --- | --- |
| `changelog/desktop-v<target>.md` | The desktop record: a lead paragraph, then `### macOS`, `### Windows` and `### Linux` |
| `CHANGELOG.md` | Link to it, at the top of the `## Desktop — macOS + Windows + Linux` list (newest first) |

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

### Linux

#### <same shape>
```

Rules:

- A patch's file has the lead paragraph and ONLY its platform's `###` section;
  the release page shows what this version ships, and the other platforms ship
  nothing in it.
- The file must be **committed** before either platform publishes: the release
  body is read out of the tagged commit, not the working tree, and a version
  with no changelog in that commit fails the publish outright.
- English only: UI labels by their i18n `en` value (look up `"hanji": "<label>"` in `i18n/*.json`), quotes translated; CJK / TL / POJ / TPS only for Taigi example words and readings.
- This is the GitHub release body users read: concrete user-visible behavior,
  with the PR number in `(#NNN)`. No refactors, tests, tooling, or dependency
  bumps unless a user feels them.
- A subsection per surface, not one flat list — these bodies run long and the
  headings are what make them readable.
- Idempotent: re-running on an existing target file merges by topic. Refine the
  line for a behavior; never add a second line for it.
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

Deterministic validation does not replace the user's factual review.

## 6. Commit

```bash
git add -A
git commit -m "desktop v<target>: release prep + changelog"
git push origin main
```

Report the commit SHA, the release range, and every rendered section. Ask the
maintainer to read the changelog back before staging: deterministic validation
cannot tell whether a sentence describes the behavior that shipped.

## 7. Stage all three installers

```bash
make desktop-release
```

`scripts/stage-desktop.sh` runs `make macos-release` here — build, sign,
notarize, package, put the `.pkg` and its `.sha256` on the **draft**
`desktop-<target>` — then dispatches `.github/workflows/windows-build.yml` and
`.github/workflows/linux-build.yml` (with the staged commit as `source_sha`)
together and waits for both: GitHub-hosted runners build the installer and the `.deb` from
the same commit and attach them, each with its `.sha256`, to the same draft.
The maintainer's Windows box is not in the release path.

A draft has no tag and no public asset URL: nothing here reaches a user, and
nothing is announced. The tag appears when the draft is published.

A full release is all three or none — no half of one is ever staged. An existing draft for
this version is deleted first, so a re-run is a fresh build of every half from
one commit — which is what the tag on the published release will describe. If a
half fails, or `main` moved while a half was waited on, fix that and run the
whole thing again.

For a **patch**, stage only its platform:

```bash
make desktop-patch PLATFORM=<platform>
```

Same script, same clean draft from one commit, with one platform's installers
(Linux: the `.deb`, `.rpm` and Arch package together). Without macOS the script
creates the empty draft itself before dispatching, since the hosted attach
steps only join one. Publishing announces only that platform; the others keep
offering the version they have.

Report the draft URL.

## Hand off — the one manual step

**Test what was staged, then publish it** — on the draft's own page, which step
7 printed. A draft is visible in the web UI to anyone who can write this
repository: the three installers are download links there, and **Publish release** is a
button on the same page. This is the decision the draft exists to protect, so the
skill never presses it.

Install each (the `.deb` on the Linux VM, S74), run the dogfood checklist items
this release touches, then publish.
(The same two steps from a terminal, if that is closer to hand:
`gh release download desktop-<target> --repo taigikeyboard/taigikeyboard --dir ~/Downloads`
and `gh release edit desktop-<target> --repo taigikeyboard/taigikeyboard --draft=false`.)

Publishing creates the tag and fires three workflows:
`.github/workflows/announce-release.yml`, which proves every download is
anonymously reachable, writes every `_data/*_release.json` to the website in one
commit and waits for the live macOS and Windows appcasts (Linux has none);
`.github/workflows/windows-build.yml`, a GitHub-hosted rebuild for SignPath
provenance; and `.github/workflows/linux-build.yml`, the same rebuild for the
`.deb`. The two rebuilds publish nothing.

The website's Linux button stays hidden until `enable_linux_download` is `true`
in the website's `_config.yml` — that switch is the maintainer's, like the
publish. It hides the button, not the `.deb`, which is public once published.

`make desktop-announce` runs the same announcement by hand — for a re-run after
a failed job, or when its token has expired. Full procedure and rationale:
`docs/architecture/desktop-release.md`.

## Guardrails

- Never edit another version's changelog.
- Never touch `changelog/mobile-v<version>.md` or `changelog/store/**` — that is
  `release-mobile`'s surface, and desktop-only work must never enter a store note.
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
