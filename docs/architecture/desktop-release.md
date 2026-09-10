# Desktop release — one draft, two machines, published by hand

How a desktop version (macOS + Windows, one shared number) gets from a commit to
a user. The platform-specific halves are `macos-release.md` (certificates,
notarization, the package) and `windows-release.md` (signing status, the
installer, the box); everything below is shared by both, and is the single
description of the flow — the platform documents link here rather than repeat it.

## The flow

A desktop release happens in two halves with a manual test between them, and
**nothing reaches a user until a person publishes it**:

| | Runs | Does |
|---|---|---|
| Stage the package | `make macos-release` (this Mac) | Builds, signs, notarizes, packages, then `macos/scripts/publish-release.sh` puts the `.pkg` on a **draft** release `desktop-<version>` |
| Stage the installer | `make windows-release RELEASE_FLAGS=--skip-sign` (the Windows box) | Attaches the `.exe` to the same draft |
| **Test** | the maintainer | `gh release download desktop-<version> --repo taigikeyboard/taigikeyboard --dir ~/Downloads`, install, use both |
| **Publish** | the maintainer | `gh release edit desktop-<version> --repo taigikeyboard/taigikeyboard --draft=false`, or the web UI. This is what creates the tag |
| Announce | **automatic** — publishing fires `.github/workflows/announce-release.yml` | Proves both downloads are anonymously reachable, writes both `_data/*_release.json`, waits for the live appcasts. `make desktop-announce` is the same script, for a re-run |

Staging creates no tag — publishing does — and a draft has no public asset URL,
so no user, no search engine and no installed copy can reach what is staged. A
tag or a draft target left by an earlier attempt is moved onto the commit being
staged, so re-staging never has to be untangled by hand; a release that has
already been **published** is the exception and stops the run.

Beside each installer goes a `.sha256` of what was staged. On the unsigned
Windows channel it is what a user can check a manual download against, and it is
what `windows-release.md` promises every release publishes.

The release goes in **this** repository; only the website's own data goes to
`taigikeyboard/taigikeyboard.github.io`:

| What | Where | Why there |
|---|---|---|
| Each installer and its `.sha256` | assets on the GitHub release `desktop-<version>` **in this repository**, both platforms on one release | One desktop version is one release, beside the source it was built from: the tag names that commit, the notes are that commit's changelog. Releases lived on the website repository while this one was private and nothing served from it was anonymously reachable; it has been public since 2026-09-07. Release assets live outside git either way, so they cost no repository its size or bandwidth allowance. |
| `_data/{macos,windows}_release.json` | committed site data in the website repository — written only by the announcement, both in one commit | The landing page's macOS download button reads it and links straight at the package, so its URL carries the version. Keeping it as data the release flow writes is what stops the page hard-coding a version, and what keeps the button off `/releases/latest` — that alias is repository-wide, and this repository's last release may be a Windows installer. |
| `appcast/{macos,windows}.json` | **rendered** from that data by the site's own build, served from `https://taigikeyboard.tw/appcast/` | Every installed copy has its URL baked in (`UpdateChecker.publishedURL`, `manifest::PUBLISHED_URL`) and expects a fixed shape, so the manifest stays a static file on the project's own domain rather than anything GitHub serves. Rendered rather than written because two files meant two commits, and two Pages runs seconds apart deploy their own trees: see *One published fact, one committed file* in `macos/updates/README.md` for the day the manifest sat a release behind. |

Each platform's `publish-release.sh` owns only what that OS can assert about its
own artifact; everything above it is shared (see *Where it all lives* below).

### What staging checks, in order

1. The shared preconditions, before any evidence is gathered about the artifact,
   so a mistake costs a second rather than a notarization wait or a signtool
   round trip: `gh` present and authenticated, a dotted-integer version, a clean
   tree, HEAD an ancestor of `origin/main`, and `changelog/desktop-v<version>.md`
   present **in that commit**. Not `main`'s tip — `main` may move between the two
   platforms' staging runs, and requiring the tip would strand whichever runs
   second.
2. What only that platform can assert about its own artifact: notarization,
   Gatekeeper and the package's declared identity on macOS
   (`macos-release.md`); the installer's name, VERSIONINFO and Authenticode
   state on Windows (`windows-release.md`). Both exist because the path argument
   can point at any file, while the tag and the manifest version come from the
   checkout — a stale artifact would otherwise be staged under the current
   version's name.
3. Whatever already exists for this version is pointed at this commit. Re-staging
   is the normal case — a fix, a second attempt, the other platform running a day
   later — so a draft's `targetCommitish` is rewritten to this exact SHA (never a
   branch name, which would tag whatever that branch points at on publish day),
   and a tag left by a hand-push or an earlier attempt is force-moved through the
   git refs API, since `gh release create` ignores `--target` once a tag exists.
   The one thing that is refused rather than moved is a release that is already
   **published**: its installers are downloadable, so moving its tag would rewrite
   what a version people already have means, and adding an asset to it would go
   public with no test. Put it back in draft, or cut a new version.
4. `gh release create --draft --target <commit>` with the whole
   `changelog/desktop-v<version>.md` as the notes (both platforms share the
   page, so both sections belong on it), or an upload into the existing draft.
   Never `--clobber`, and never over an asset already staged: an identical one
   is verified in place, a differing one stops the run.
5. The staged asset is downloaded back — authenticated, since a draft has no
   anonymous URL — and its SHA-256 compared to the local file's, so what the
   maintainer is about to test is provably what was built.

### What announcing checks, in order

1. The release is published, not a draft. A manifest naming a draft points every
   installed copy at a download that does not exist.
2. For each platform's installer on it: the asset is fetched **with no
   credentials at all** — `curl -q --netrc-file /dev/null` is what guarantees
   that; an authenticated check cannot tell a public URL from a private one,
   which is how the first version of this shipped pointing at a private
   repository — and hashed. That hash is what the Windows manifest publishes, so
   the digest it names is one the URL was observed serving.
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

### One-time: the announcement's token

The announcement commits to the website repository, which a workflow's own
`GITHUB_TOKEN` cannot reach. One fine-grained personal access token covers it:

- Repository access: `taigikeyboard/taigikeyboard` and
  `taigikeyboard/taigikeyboard.github.io`.
- Permissions: **Contents: read** on the first (the script reads the release),
  **Contents: write** on the second (it writes `_data/*_release.json`).
- Stored as the `SITE_CONTENTS_TOKEN` secret on `taigikeyboard/taigikeyboard`.

Give it an expiry and let it lapse: the day it does, the workflow fails loudly
and `make desktop-announce` from a machine with `gh` logged in does the same
job, so a release is never blocked on it.


## Where it all lives

| Piece | File |
|---|---|
| The release object: preflight, draft, tag alignment, create-or-attach, read-back | `scripts/lib/desktop-release.sh` |
| The website: anonymous fetches, the one-commit site write, the manifest poll | `scripts/lib/release-site.sh` |
| The announcement, run by the publish | `scripts/announce-release.sh` + `.github/workflows/announce-release.yml` |
| What only a Mac can say about the package | `macos/scripts/publish-release.sh` |
| What only Windows can say about the installer | `windows/scripts/publish-release.sh` |
| The manifest wire formats | `macos/updates/README.md`, `windows/updates/README.md` |
