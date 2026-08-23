# macOS update manifest

The macOS input method checks for updates by fetching one static JSON file:

    https://taigikeyboard.tw/appcast/macos.json

It compares `version` against the installed `CFBundleShortVersionString` and,
when the manifest is strictly newer, offers to open `downloadPageURL` in the
browser. Notify-only: nothing is downloaded or executed, which is why the file
needs no signing — HTTPS plus a version comparison is the whole contract.

The reader is `macos/Sources/TaigiInputMethodCore/Settings/UpdateChecker.swift`.

## Where it lives

The file itself is **not** in this repository. It is served from the website
repository, `taigikeyboard/taigikeyboard.github.io`, at `appcast/macos.json`,
and `macos/scripts/publish-release.sh` is what writes it there.

Two constraints put it there rather than here:

- **This repository is private.** Anything served from it — `raw.githubusercontent.com`
  or a releases page — answers an anonymous request with `404`. A manifest here
  is a manifest no user can read.
- **`UpdateChecker.publishedURL` is compiled into every shipped build** and old
  installs request it forever. Only a URL on a domain the project controls can
  be repointed at different hosting later without stranding them.

Keeping a second copy here to review would only give it somewhere to drift, and
a hand-edited manifest can go live before the package it announces exists. The
publish script generates it instead, after the download is verified reachable.

## Wire format

| Field | Meaning |
|---|---|
| `version` | Newest downloadable version, dotted integers only (`3.6.5`). No suffixes — the checker rejects them. |
| `downloadPageURL` | Page the user lands on, `https` only. A page, not a file: the user downloads and runs the pkg themselves. |

Unknown extra fields are ignored by old installs, so the format can grow.

`downloadPageURL` arrives *in* the manifest, so unlike `publishedURL` it is not
baked into any build and can be repointed at any time — at a download page on
the website, for instance, once one exists.

Before the first release the manifest reads `0.0.0`, which is older than every
build in existence and therefore notifies nobody. That is what "nothing has been
published yet" looks like on the wire.

## Publishing a release

`macos/scripts/publish-release.sh` (or `make macos-release RELEASE_FLAGS=--publish`,
which runs it straight after a successful build) does the whole sequence:

1. Checks the package really is this app at this version, then uploads it as a
   GitHub release asset on the website repository — release assets, unlike
   committed files, do not count against the 1 GB GitHub Pages site limit or its
   bandwidth allowance, and never enter the site's git history.
2. Re-fetches the release page and the asset **anonymously**, with no GitHub
   credentials, and requires both to answer `200`.
3. Only then writes `appcast/macos.json`, and waits for the live URL to serve
   the new version.

Re-running it after a failure is the intended recovery — it adds to an existing
release rather than replacing it, so a published download is never taken away
while the manifest still points at it.

The order is the point. A manifest published before its download is reachable
points every checker at a 404, and a developer's own browser cannot see that
happening — it is authenticated.

See `docs/architecture/macos-release.md` for the one-time account setup and the
full release procedure.
