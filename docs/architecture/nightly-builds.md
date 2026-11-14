# Nightly builds — unsigned desktop + ChromeOS builds of `main`

Every night `.github/workflows/desktop-nightly.yml` builds `main` for macOS,
Windows, Linux and ChromeOS and overwrites the files at
`https://nightly.taigikeyboard.tw/latest/`. The website's Nightly download group
links them. A nightly build is a test build, not a release: it has no draft, no
tag, no signature and no appcast entry, and nothing installed is ever offered one
as an update. The release flow (`desktop-release.md`) is unchanged by it.

## What runs

| Job | Holds | Does |
|---|---|---|
| `plan` | `contents: read` | `tools/release/nightly-release.sh check` reads the live `build.json` anonymously. If it already names `main`, nothing is built (`force` overrides). A 404, a network error or an unreadable file means build |
| `macos` / `windows` / `linux` / `chromeos` | `contents: read` in the jobs that run | The same four build workflows the train calls, with `build_only: true` and `source_sha` = `main` |
| `macos-package` | no secret | `macos/scripts/package-nightly.sh` wraps the ad-hoc signed bundle in an **unsigned** `.pkg`, using the same layout as a release (`macos/scripts/lib/installer-package.sh`, shared with `release-app.sh`), and fails if the package carries a signature |
| `publish` | `nightly` environment | Runs only when every platform succeeded and the run is on `main` without `dry_run`. Renames each installer to its fixed name, uploads it with a `.sha256` receipt and `Cache-Control: no-cache`, reads it back anonymously, then writes `build.json` (`commit`, `version`, `built`) last |

The schedule is 00:41 Asia/Taipei. `windows-build.yml` cancels an in-progress run
of the same commit, so do not stage a hotfix at that hour.

| File | Install |
|---|---|
| `TaigiKeyboard-nightly.pkg` | Unsigned. macOS blocks the first open; go to **System Settings → Privacy & Security → Open Anyway**. The Installer does not carry the download's quarantine onto the installed input method |
| `TaigiKeyboard-nightly.exe` | Unsigned, like the current release installer; SmartScreen asks once |
| `taigikeyboard-nightly_amd64.deb` | `sudo apt install --reinstall ./taigikeyboard-nightly_amd64.deb`. Without `--reinstall`, apt skips a package whose version is already installed |
| `taigikeyboard-nightly.x86_64.rpm` | `sudo dnf reinstall ./taigikeyboard-nightly.x86_64.rpm` (or `install` when it is not installed) |
| `taigikeyboard-nightly-x86_64.pkg.tar.zst` | `sudo pacman -U` reads the package metadata, not the file name, and reinstalls the same version |
| `TaigiKeyboard-nightly-chromeos.zip` | Unzip it, then use **Load unpacked** in `chrome://extensions` with Developer mode on. The manifest has no `key`, so this copy gets its own extension ID: its settings and learned words are separate from the store copy |

## Version

A nightly build carries the version in the tree, which is the last published
desktop version (only the train's cut commit changes it). The in-app update
check (`desktop/crates/taigi-desktop-update/src/checker.rs`) offers an update
only when the remote version is higher, so a nightly build is offered the next
release. The package managers see the same version from one night to the next,
which is why the Linux commands above reinstall. On macOS the package's build
version (`CFBundleVersion`) is also the same from night to night, and the
component is version-checked (`BundleIsVersionChecked`,
`macos/scripts/lib/installer-package.sh`): installing one nightly over another
of the same build version is the first on-device check of this channel, not yet
verified.

## One-time setup (maintainer)

1. Cloudflare: create the R2 bucket `taigikeyboard-nightly` and connect the custom domain `nightly.taigikeyboard.tw`. That domain sits one level under the zone, so the free Universal SSL certificate covers it. Check that no cache rule meant for `dl.taigikeyboard.tw`'s 30-day caching also matches `nightly.`, because such a rule would override `no-cache`.
2. Create an R2 API token with Object Read & Write on **only** `taigikeyboard-nightly`. It cannot write the release bucket `taigikeyboard-downloads`.
3. In GitHub, create the environment `nightly`: deployment branches `main` only, no required reviewer. Add the secrets `NIGHTLY_R2_ACCESS_KEY_ID`, `NIGHTLY_R2_SECRET_ACCESS_KEY` and `R2_ACCOUNT_ID`.

## Cost

Everything stays inside free tiers (checked 2026-11-14):

- GitHub Actions: the repository is public, so standard hosted runners and artifact storage cost nothing.
- R2 free tier: 10 GB-month of storage, 1M Class A operations, 10M Class B operations, and free egress, counted per account. The fixed names keep about 290 MB stored. Uploads take about 60 Class A operations a night (files over 8 MB go up in multipart), which is about 1,800 a month.
- A night whose `main` has not changed builds nothing.

## Trying it

- `gh workflow run desktop-nightly.yml --ref <branch> -f dry_run=true` builds and packages without uploading. A dispatch from any ref other than `main` is always a dry run.
- `gh workflow run desktop-nightly.yml -f force=true` rebuilds and re-uploads an unchanged `main`.
