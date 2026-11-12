# Desktop release — one draft, every desktop build, published by hand

How a desktop version (macOS + Windows + Linux + ChromeOS, one shared number) gets from a commit to
a user. The platform-specific halves are `macos-release.md` (certificates,
notarization, the package), `windows-release.md` (signing status, the
installer, the box), `linux-release.md` (the `.deb` / `.rpm` / Arch packages)
and `chromeos-release.md` (the Chrome Web Store zip); everything below is
shared by all four, and is the single
description of the flow — the platform documents link here rather than repeat it.

## The flow

A desktop release happens in two halves with a manual test between them, and
**nothing reaches a user until a person publishes it** — apart from the
unlisted beta page, which the stage fills on its own:

| | Runs | Does |
|---|---|---|
| Stage all | `make desktop-release` (this Mac) | Builds, signs, notarizes and stages the `.pkg` here, then dispatches `windows-build.yml`, `linux-build.yml` and `chromeos-build.yml` together and waits for each: GitHub-hosted runners build the `.exe`, the Linux `.deb` / `.rpm` / Arch packages and the ChromeOS Web Store zip from the same commit and attach them to the same draft. Every platform or none, onto a draft it re-creates each run (`tools/release/stage-desktop.sh`; the Linux half: `linux-release.md`; the ChromeOS half: `chromeos-release.md`) |
| Stage a patch | `make desktop-patch PLATFORM=macos\|windows\|linux\|chromeos` (this Mac) | The same run for ONE platform: its installers only, on a draft of their own version. Without macOS, the script creates the empty draft on this commit first, since the hosted attach steps only join one |
| Beta | **automatic** dispatch of `desktop-beta.yml` at the end of a successful stage; runs at once (the `release` environment has no reviewer since 2026-11-12: the stage command and the publish are the human steps; only `main` and ruleset-protected `*-*.*.*` tags may use its secrets) | Copies the draft's installers to R2 `beta/<version>-<sha7>-<run id>/` and lists them on the unlisted `https://taigikeyboard.tw/beta` (`noindex`, not in the sitemap, linked from nowhere) for internal testers, who cannot open a draft. Only the newest beta is kept; the page hides a beta once its version is published (`tools/release/beta-release.sh`). Once the live page lists the build, the run posts the page link, each installer's R2 link and the release-on-the-5th note to Discord #announcements, with link previews suppressed through the `DISCORD_BETA_WEBHOOK` secret (`release` environment); a failed post only warns. A failed run leaves the page with what it had. A beta and an announcement committing to the website at once: the loser fails loudly, and re-running it is the fix |
| **Test** | the maintainer | Open the draft's page — a draft is visible in the web UI to anyone who can write the repository — download the assets, install, use them |
| **Publish** | the maintainer | **Publish release** on that same page (or `gh release edit desktop-<version> --draft=false`). This is what creates the tag |
| Announce | **automatic** — publishing fires `.github/workflows/announce-release.yml` | Proves each download is anonymously reachable, writes every `_data/*_release.json` in one commit, waits for the live macOS and Windows appcasts (Linux and ChromeOS have none). `make desktop-announce` is the same script, for a re-run |
| Upload to the Chrome Web Store | the maintainer, by hand — only after a release carrying the ChromeOS zip is published (the announcement prints the reminder) | Uploads `TaigiKeyboard-<version>-chromeos.zip` from the release in the Developer Dashboard and submits it for review; the store, not the release, is how users install and update it (`chromeos-release.md` § Upload) |
| Bump the Homebrew cask | the maintainer, by hand — only after a release carrying a macOS `.pkg` is published | Sets `version` and `sha256` (from the release's `TaigiKeyboard-<version>.pkg.sha256`) in `Casks/taigikeyboard.rb` of `taigikeyboard/homebrew-tap`, then `brew audit --cask --strict --online taigikeyboard/tap/taigikeyboard`. A Windows-, Linux- or ChromeOS-only patch leaves the cask alone |

### Version numbers: full releases and patches

A **full release** bumps the minor number and stages every platform: 3.7.0,
3.8.0. A **patch** bumps the third number and stages only the platform it fixes:
3.7.1 macOS, 3.7.2 Windows. The patch counter is shared, so each
patch is one version, one tag, one draft; a platform skips the numbers it did not
ship (Windows goes 3.7.0 → 3.7.2) and its update check only compares, so the gap is
harmless. Announcing a patch writes only its platform's `_data` / appcast, so the
others keep offering the version they have. The next full release lines every
platform up again. Its changelog `changelog/desktop-v<version>.md` has only the
patched platform's section. Setting the number, and why minor and patch each stop
at 99: [`manual-release-notes.md` § Set the version](manual-release-notes.md#set-the-version).

Staging creates no tag — publishing does — and a draft has no public asset URL,
so no user, no search engine and no installed copy can reach the draft itself.
The beta page's copy is public to anyone holding its URL, never named by an
appcast, a download button or the cask.
`make desktop-release` / `make desktop-patch` delete an existing draft for the
version first and rebuild everything on it from one commit
(`tools/release/stage-desktop.sh`); a version that has already been
**published** stops the run.

Beside each installer goes a `.sha256` of what was staged. On the unsigned
Windows channel — and the Linux packages, which are not signed either — it is what a
user can check a manual download against, and it is what `windows-release.md`
and `linux-release.md` promise every release publishes.

The release goes in **this** repository; only the website's own data goes to
`taigikeyboard/taigikeyboard.github.io`:

| What | Where | Why there |
|---|---|---|
| Each installer and its `.sha256` | assets on the GitHub release `desktop-<version>` **in this repository**, every desktop platform on one release | One desktop version is one release, beside the source it was built from: the tag names that commit, the notes are that commit's changelog. Release assets live outside git, so they cost the repository no size or bandwidth allowance. |
| The download mirror: each installer and its `.sha256` | the Cloudflare R2 bucket `taigikeyboard-downloads`, served at `https://dl.taigikeyboard.tw/desktop/<asset>` — copied there by the announcement | GitHub's release-asset host crawls on some Taiwanese ISPs (HiNet measured ~40 KB/s on 2026-10-09, against ~16 MB/s from R2), so the website buttons and both appcasts name the mirror; the GitHub release stays the original it is checked against. See *One-time: the R2 mirror* and *Pruning the mirror*. |
| The beta copy of a draft and `_data/beta_release.json` | R2 `beta/<version>-<sha7>-<run id>/` (newest only) and the website repository, written by `desktop-beta.yml` | Internal testers have no write access to see a draft. A fresh prefix per run because the download host caches for 30 days and a re-stage at one commit builds different bytes. |
| `_data/{macos,windows,linux,linux_rpm,linux_arch,chromeos}_release.json` | committed site data in the website repository — written only by the announcement, all in one commit (`tools/release/lib/release-site.sh`) | The landing page's download buttons read them, and the /beta page hides a beta whose version one of them names (the ChromeOS file exists for that alone, since the store distributes the zip); the macOS one and links straight at the package, so its URL carries the version. Keeping it as data the release flow writes is what stops the page hard-coding a version, and what keeps the button off `/releases/latest` — that alias is repository-wide, and this repository's last release may be a Windows installer. |
| `appcast/{macos,windows}.json` | **rendered** from that data by the site's own build, served from `https://taigikeyboard.tw/appcast/` | Every installed copy has its URL baked in (`UpdateChecker.publishedURL`, `manifest::PUBLISHED_URL`) and expects a fixed shape, so the manifest stays a static file on the project's own domain rather than anything GitHub serves. Rendered rather than written, so each release is one committed file per platform: [`macos/updates/README.md` § One published fact, one committed file](../../macos/updates/README.md#one-published-fact-one-committed-file). |
| `Casks/taigikeyboard.rb` | the `taigikeyboard/homebrew-tap` repository (`brew install --cask taigikeyboard/tap/taigikeyboard`) | Homebrew finds a tap only in a repository named `homebrew-*`. The cask names the last **published** macOS `.pkg` — never a draft or the in-tree version — and its `livecheck` counts only published releases carrying a `TaigiKeyboard-<version>.pkg` asset, so a Linux-only patch release or a `mobile-*` tag never looks like a new macOS version. Its `url` is the R2 mirror (`https://dl.taigikeyboard.tw/desktop/`), so a bump must follow the announcement that mirrors the package; `livecheck` names the GitHub repository explicitly. The app's own update check still applies (`auto_updates true`). |

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
3. Whatever already exists for this version is pointed at this commit. Under
   `make desktop-release` the draft was just deleted, so this only matters when a
   platform's `publish-release.sh` is run directly: a draft's `targetCommitish`
   is rewritten to this exact SHA (never a branch name, which would tag whatever
   that branch points at on publish day), and a tag left by a hand-push or an
   earlier attempt is force-moved through the git refs API, since
   `gh release create` ignores `--target` once a tag exists. A release that is
   already **published** is refused rather than moved: its installers are
   downloadable, so moving its tag would rewrite what a version people already
   have means. Immutable releases (repository setting, on since 2026-11-04) lock
   its tag and assets too, so the only way forward is a new version.
4. `gh release create --draft --target <commit>` with the whole
   `changelog/desktop-v<version>.md` as the notes (every platform shares the
   page, so every section belongs on it), or an upload into the existing draft.
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
   repository — and hashed, and its staged `.sha256` receipt is held to that
   hash. Both are then copied to the R2 mirror and the installer is read back
   from `https://dl.taigikeyboard.tw/desktop/<asset>`, again anonymously, and
   must hash the same. That mirror URL is what the site data and appcasts name,
   and the hash is what the Windows manifest and the Linux site data publish, so
   the digest each names is one the mirror was observed serving.
3. Every platform's data file is written to the website in **one** commit, so
   they cannot race each other's Pages deployment.
4. Each live appcast (macOS, Windows) is polled until it serves this version **and** its package
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

`macos/updates/README.md` and `windows/updates/README.md` document the manifest
wire formats.

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


### One-time: the R2 mirror

The announcement uploads to the R2 bucket `taigikeyboard-downloads` through R2's
S3-compatible API with the AWS CLI (preinstalled on `ubuntu-latest`; `brew
install awscli` for a local run):

- In the Cloudflare dashboard, R2 → Manage API tokens → **Account API token**,
  permission **Object Read & Write**, bucket `taigikeyboard-downloads` only.
- Stored as three secrets on `taigikeyboard/taigikeyboard`: `R2_ACCESS_KEY_ID`,
  `R2_SECRET_ACCESS_KEY`, `R2_ACCOUNT_ID` (the id in the S3 endpoint
  `https://<id>.r2.cloudflarestorage.com`). A local `make desktop-announce`
  exports the same three.
- The bucket uses the **Standard** storage class — Infrequent Access is outside
  the free tier — and its custom domain is `dl.taigikeyboard.tw`; the `r2.dev`
  URL stays disabled.

### Pruning the mirror

The announcement only adds to `desktop/`. (The beta run deletes older `beta/`
prefixes itself; it never touches `desktop/`.) Removing a superseded version is a
manual step, after its successor is announced on every platform: a manifest an
installed copy has already fetched, or a cached download page, can still name
the old object, so deleting it at announce time would turn those into failed
downloads. List what the six `_data/*_release.json` files name, and delete
every other `desktop/` object (and its `.sha256`) with `aws s3 rm` against the
same endpoint. GitHub keeps every release, so older versions stay downloadable
there.

## Where it all lives

| Piece | File |
|---|---|
| The release object: preflight, draft, tag alignment, create-or-attach, read-back | `tools/release/lib/desktop-release.sh` |
| The website: anonymous fetches, the R2 mirror (`require_r2`, `r2`), the one-commit site write, the manifest poll | `tools/release/lib/release-site.sh` |
| The announcement, run by the publish | `tools/release/announce-release.sh` + `.github/workflows/announce-release.yml` |
| What only a Mac can say about the package | `macos/scripts/publish-release.sh` |
| What only Windows can say about the installer | `windows/scripts/publish-release.sh` |
| The Linux package and its attach step | `linux/Makefile` (`deb` / `rpm` / `arch`), `.github/workflows/linux-build.yml`, `linux-release.md` |
| The manifest wire formats | `macos/updates/README.md`, `windows/updates/README.md` |
