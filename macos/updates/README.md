# macOS update manifest

The macOS input method checks for updates by fetching one static JSON file:

    https://taigikeyboard.tw/appcast/macos.json

It compares `version` against the installed `CFBundleShortVersionString` and,
when the manifest is strictly newer, offers the update: from the settings window
the app downloads `packageURL` itself and opens it in Installer.app, and
everywhere else — the system notification, an old manifest, a development build
— it opens `downloadPageURL` in the browser.

The file is still unsigned, and that is deliberate now that something it names
gets executed. Whoever could rewrite this manifest could name any package;
what stops that package being installed is its own Developer ID signature,
checked against the team that signed the running copy. Signing the manifest as
well would add a second key to protect and would be read by the same code that
already has to distrust it. Gatekeeper cannot make this call alone — it accepts
any correctly notarized package, including one an attacker notarized under their
own Developer ID — which is why the team is pinned rather than inferred.

The readers are `macos/Sources/TaigiInputMethodCore/Settings/UpdateChecker.swift`
(the check) and `UpdateInstallation.swift` / `UpdatePackageVerifier.swift` (the
download and what it has to prove).

## Where it lives

The file itself is **not** in this repository. It is served from the website
repository, `taigikeyboard/taigikeyboard.github.io`, at `appcast/macos.json`.

What puts it there rather than here: **`UpdateChecker.publishedURL` is compiled
into every shipped build** and old installs request it forever, so the manifest
has to sit at a fixed URL on a domain the project controls — one that can be
repointed at different hosting later without stranding them. The package it
announces is a release asset in this repository; the manifest is not.

(There was a second reason until 2026-09-07: this repository was private, so
nothing served from it answered an anonymous request with anything but `404`.
That is why the releases lived over there too. They are back here; the manifest
stays where every installed copy already looks for it.)

Keeping a second copy here to review would only give it somewhere to drift, and
a hand-edited manifest can go live before the package it announces exists.

### One published fact, one committed file

A release writes exactly one file over there, `_data/macos_release.json`:

```json
{
  "version": "3.6.6",
  "tag": "desktop-3.6.6",
  "downloadURL": "https://github.com/taigikeyboard/taigikeyboard/releases/download/desktop-3.6.6/TaigiKeyboard-3.6.6.pkg",
  "releasePageURL": "https://github.com/taigikeyboard/taigikeyboard/releases/tag/desktop-3.6.6"
}
```

That file is what the site's macOS download button links at — straight at the
package so the download starts on one click, which is why its URL carries the
version, and deliberately not `/releases/latest/download/...`, since `latest`
resolves repository-wide and the last release here may be a Windows one.

The manifest is **rendered** from it by the site's own build:
`appcast/macos.json` over there is a Jekyll template reading
`site.data.macos_release`, mapping `releasePageURL` → `downloadPageURL` and
`downloadURL` → `packageURL`. Nothing writes it directly.

It was two literal files until 2026-08-29, each committed by the publish script
in its own commit. **Two commits seconds apart race.** Every GitHub Pages run
deploys the tree of *its own commit*, not the branch tip, so whichever run
finishes last wins — and on 2026-08-28 that was the run for the earlier commit.
The site's download button read 3.6.6 while the manifest served 3.6.5, for a
day, with both files correct on `main` the whole time. The publish script's
liveness poll did not catch it: the manifest genuinely was live at 3.6.6 when it
looked, and was reverted afterwards.

One file cannot race with itself, so that is the shape now. It also removes the
duplication that made the two files drift-capable in the first place: `version`
was in both, and `downloadURL`/`packageURL` and `releasePageURL`/`downloadPageURL`
were the same two URLs under different names.

## Wire format

| Field | Meaning |
|---|---|
| `version` | Newest downloadable version, dotted integers only (`3.6.5`). No suffixes — the checker rejects them. |
| `downloadPageURL` | Page the user lands on, `https` only. Required: it is the only route a system notification, a development build, or a pre-3.6.6 install has. |
| `packageURL` | The `.pkg` itself, `https` only. Optional. Naming it lets the app download and install without a browser; a manifest without it behaves exactly as manifests did before 3.6.6. |

Unknown extra fields are ignored by old installs, so the format can grow. A
`packageURL` that fails validation is dropped on its own rather than failing the
whole manifest — it carries an added convenience, and one mistake in it must not
be able to silence update checking for every installed copy.

`downloadPageURL` arrives *in* the manifest, so unlike `publishedURL` it is not
baked into any build and can be repointed at any time — at a download page on
the website, for instance, once one exists.

Before the first release the manifest reads `0.0.0`, which is older than every
build in existence and therefore notifies nobody. That is what "nothing has been
published yet" looks like on the wire.

## Publishing a release

`macos/scripts/publish-release.sh` — which `make macos-release` runs straight
after a successful build — does the whole sequence:

1. Checks the package really is this app at this version, then puts it on the
   `desktop-<version>` release in this repository — the one the Windows
   installer shares — creating it (tagging the commit this checkout is at) or
   attaching to what the Windows publish already created.
2. Re-fetches the release page and the whole asset **anonymously**, with no
   GitHub credentials, and requires `200` and a SHA-256 equal to the local
   package's, which is what proves the upload landed whole.
3. Only then writes `_data/macos_release.json` — one commit — and waits for the
   live manifest URL to serve the new version. That wait is also what proves the
   site rendered the manifest from what was committed, which is the one step of
   the announcement the script no longer performs itself.

Re-running it after a failure is the intended recovery — it adds to an existing
release rather than replacing it, so the release the manifest points at is never
torn down and rebuilt, and an asset already published is verified in place
rather than re-uploaded.

The order is the point. A manifest published before its download is reachable
points every checker at a 404, and a developer's own browser cannot see that
happening — it is authenticated.

See `docs/architecture/macos-release.md` for the one-time account setup and the
full release procedure.
