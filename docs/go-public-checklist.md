# Going public — checklist

This repository is private and will be made public at a date that has not been
set. The flip is irreversible in practice: forks, caches and archives keep
whatever the history contained at that moment. This file is the ordered list of
what has to be true first, how each item was checked, and when — so the flip is
a mechanical step rather than a judgement call made in a hurry.

Re-run each audit rather than trusting the recorded result; the dates say how
stale it is.

---

## 1. No credential found in history

**No findings** — gitleaks 8.30.1 over the full history, re-run 2026-09-07
against the rewritten history, `main` at `1b00a9cc`, which `.gitleaks-scanned`
now records. One scanner under one configuration; that is evidence, not proof.

```sh
make scan-secrets-full
```

1135 commits and 858 MB scanned. Five findings, all read and dismissed:

| Match | Verdict |
| --- | --- |
| `Tab1Fragment.kt` ×3 — a `titleKey` argument holding `faq_1_question` | Localization key constants; the rule fires on the shape of a variable named "key" being assigned a string. |
| `臺灣方音符號.html` ×2 — `wgConfirmEditHCaptchaSiteKey` | A third party's hCaptcha **site** key inside a saved copy of a public web page. Site keys are published in page source by design; the secret counterpart is the secret key, which is not here. |

Five rather than the eight recorded before: the `StringKey.swift` matches lived
in objects the history rewrites dropped.

⚠ **A gitleaks fingerprint is `<commit>:<path>:<rule>:<line>` and cannot survive a
history rewrite.** After 2026-09-07's two rewrites every fingerprint in
`.gitleaksignore` was stale and all five findings resurfaced, while
`.gitleaks-scanned` named a commit that no longer existed — so the incremental
scan had no base and silently fell back to a full sweep every run. Neither
failure is loud. Both were refreshed; **re-check them after any future rewrite.**

## 2. Personal data — what was checked

Not a clean bill of health: personal data has no scanner, and what follows is a
list of specific things looked for, not a conclusion that none exists.

**The known exposure was real and is now resolved.** The bug-report backlog
carrying four reporters' email addresses and device models *was* committed. A
`git filter-repo` pass on 2026-09-05 removed it from every branch and tag, but
force-pushing does not purge `refs/pull/*`: walking back from closed pull request
#690's head still reached three commits that contained it, and
`git fetch origin 'refs/pull/*/head:refs/pull/*'` is one command any visitor can
run on a public repository.

Resolved 2026-09-07 by migrating to a fresh repository rather than waiting on
GitHub support to garbage-collect. The old one is `taigikeyboard-archive`,
private. Verified on the new one with a clean mirror clone:

| Check | Result |
| --- | --- |
| refs | 1 head, 33 tags, **0 `refs/pull`** |
| the file, across every ref's history | 0 commits |
| whole-object sweep for its path | 0 hits |
| the three commits, by SHA, through the API | 404 / 404 / 404 |

A second rewrite the same day replaced a work email (`alex.su@btse.com`, author
and committer on 14 commits from 2025-11/12) with the maintainer's own, and the
placeholder author name `--get-all` with a real one. Every commit's tree was
compared before and after: all 1222 identical, so no file content changed.

Also checked, with no hits: every RFC-1918 private address range across the whole
history, and email-shaped strings across the tracked tree — where every match was
`git@github.com:` in a clone script or an `@3x.png` asset filename.

Not checked: IPv6 local addresses, personal names, physical addresses, and
anything inside the binary artifacts. Pull request bodies and review comments are
no longer a concern here — the new repository has only the migration's own.

## 3. Known third-party components are attributed

`THIRD_PARTY_LICENSES.md`, current as of 2026-09-05. It is an inventory kept by
hand, so it is as complete as the last person to add to it.

Covers the four bundled fonts, vendored source (`ios/Vendor/ISEmojiView`, the
androidx icon path data), the filtered SymSpell frequency list, both sibling
repositories, and the Rust, Swift and Gradle dependency sets.

## 4. The dictionary may lawfully be redistributed

**Blocked.** This is the item that decides the date.

`dictionary/LICENSE` and `dictionary/docs/SOURCES.md` cover all nine main
sources and the three supplementary sets, verified 2026-09-05. Publishing the
repository publishes the data, and two problems stand in the way.

**Four sources have no identified redistribution licence.** "Unverified" in that
table means no licence was found — not that the data is open, and not that a
government body publishing a file grants a right to reproduce it:

| Source | Rows | What is missing |
| --- | --- | --- |
| `khpoo` 齒盤補充辭典 | 4,198 | Origin and rights holder not established at all. |
| `lkk` 漢羅合用建議用字 | 58 | Same. |
| `khiin` frequency data | 8,086 | Upstream project known; that file's terms are not. |
| `char_freq_merged.txt` | — | Corpus provenance not established. |

Each needs one of: a licence identified in writing, permission from the rights
holder, replacement by a source that has one, or removal from the build. Removal
means removal from the **history** too, if the history is going public — which is
the reason to sequence this before §6 rather than after.

**The compiled dictionary is non-commercial.** `taijit` (CC BY-NC-SA 3.0 TW,
64,250 rows) is the only source carrying a NonCommercial term over data this
project ships, and the compiled artifacts merge every source into one index that
cannot be separated back into per-source rows. Already stated in `dictionary/LICENSE` and `NOTICE`;
repeated here because a public repository invites redistribution and the
constraint has to be legible to whoever redistributes.

**Two ShareAlike versions coexist.** `taihoa` and `sitbut` (CC BY-SA 4.0) and
`taijit` (CC BY-NC-SA 3.0 TW) each carry ShareAlike terms. Whether one merged
index satisfies both at once has not been analysed.

**`kungge` has no data licence at all.** 國立臺灣工藝研究發展中心 publishes only an
圖像授權 page (姓名標示-非商業性-禁止改作, no version, scoped to the 示意圖片); nothing
covers the vocabulary. This project takes 漢字 + 台羅拼音 only, so the position rests
on the same 著作權法 §10-1 reading as `kautian` below rather than on a grant. The
in-app credits screen previously showed CC BY-NC 4.0 with no source; corrected
2026-09-07.

**The `kautian` reading is the maintainer's, not the publisher's.**
`dictionary/LICENSE` argues that taking only 漢字 + 羅馬字 from a CC BY-ND source
is a Collection rather than an Adaptation. That argument is recorded so it can be
checked — it is not a permission that was granted.

The scope of this item is therefore every raw, derived, compiled and
historically committed dictionary input, not only the four rows above.

## 5. The repository can be cloned by a stranger

**Fixed** 2026-09-05, verify again before the flip.

`.gitmodules` pointed at `git@github.com:taigikeyboard/taigi-converter.git`.
SSH is not anonymous: fetching over it requires the visitor to have their own
GitHub SSH key configured, so `git clone --recursive` would have failed at the
submodule for anyone who does not. It now uses HTTPS.

Verified: the submodule repository is public, and its pinned commit
`99261d7f` is reachable anonymously. The sibling `taigi-emojis` repository is
public as well.

Re-verified 2026-09-07: `.gitmodules` uses HTTPS, and `git ls-remote` reaches
the pinned commit `99261d7f` anonymously.

⚠ **`--recursive` is not optional.** The dictionary pipeline converts every
reading through the `taigi-converter` submodule. A clone without it fails ten
minutes in with a `TypeError` in `dictionary/common/cleanup.py:201` that mentions
neither node nor submodules — the import failure makes every conversion return an
error string, which the pipeline then ingests as data. Both entry points now
refuse to start without it (#4), and `CLAUDE.md`'s bootstrap section leads with
it. `make dict` takes about 2 minutes once it can run.

Before the flip, do this from a clean machine with no GitHub credentials
configured, not from a maintainer environment that already works:

```sh
git clone --recursive https://github.com/taigikeyboard/taigikeyboard.git
```

then build from the public instructions alone.

## 6. Repository size

**Decided 2026-09-07: the history is not being rewritten.** The repository stays
at roughly 750 MB and the growth is stopped.

Measured with a trial `git filter-repo` before deciding:

| | Size |
| --- | --- |
| Today | 753 MB |
| Without the platform build artifacts | 609 MB |
| Also without the dictionary artifacts | 217 MB |

The 392 MB that would have made a rewrite worth its cost is in the dictionary
artifacts, and those are staying committed on the maintainer's instruction —
`dictionary/` is not to be touched. That left a rewrite buying 19% in exchange
for invalidating every commit SHA a second time and dropping 17 artifact-only
commits, which was judged not worth it.

Growth is stopped instead, which was the more important half:

| Merged | What stopped being committed |
| --- | --- |
| #1 | `version_snapshot` diffs releases against a 3.1 MB key set, not the 35 MB `dictionary/output/dictionary.csv` |
| #2 | the macOS and Windows font trees — 79 MB of copies `make fonts` regenerated (both trees, and that step, are since gone: the typefaces live once at `fonts/font/`) |
| #3 | the iOS and macOS xcframeworks and the Android `.so` — `librust_taigi.a` alone had been rewritten into history in 85 commits |

If this is ever reopened, history still carries ~190 MB of pipeline layouts that
no longer exist at `HEAD` (`dictionary/csv/`, `dictionary/6_台日大辭典/`,
`dictionary/5_台華線頂對照典/`); they would go in the same pass. Note that a
rewrite invalidates `.gitleaks-scanned` and every `.gitleaksignore` fingerprint —
see §1.

## 7. The release and update chain is safe to expose

Not audited. The desktop applications fetch an update manifest over the network
and install what it names, so the accounts and workflows behind that manifest
become interesting to an attacker the moment the code describing them is public.
`docs/architecture/windows-release.md` already records that the manifest digest
and the release asset are controlled by the same GitHub account, which makes the
SHA-256 an integrity check rather than a defence against a compromised account.

Before the flip, confirm:

- MFA or passkeys, and recovery paths, on the GitHub account, the domain
  registrar, DNS, SignPath, and the Apple and Google accounts.
- Publishing tokens are scoped to the minimum and carry an expiry.
- The website repository's branch protection, and who can alter a manifest.
- No workflow reachable from an untrusted pull request holds a write token or a
  signing credential. There is currently no `pull_request_target` anywhere,
  which is the main hazard; keep it that way.
- Third-party actions pinned to an immutable SHA. **Done** 2026-09-07: both
  workflows pin `actions/checkout` and `taiki-e/install-action` by commit, with the
  tag kept in a trailing comment so Dependabot's `github-actions` ecosystem can
  still bump them. `permissions: contents: read` bounds what a moved tag could
  reach but does not remove it, since a malicious action still runs with read
  access to a private tree and can shape the gate's own verdict. The gitleaks
  binary `secrets.yml` downloads was already pinned by SHA-256. Note that pinning
  `taiki-e/install-action` does not pin the `cargo-audit` / `cargo-deny` builds it
  fetches — that is a second layer, still floating.
- The certificate rollover story. The Windows updater pins a leaf thumbprint.

## 8. What else becomes public

Making a repository public exposes more than the default branch: releases and
their assets, all tags, Actions run logs and artifacts, packages, the wiki,
issues and pull requests including attachments and every review comment, and any
branch that was never merged. Confirm GitHub's current behaviour against its own
visibility documentation on the day, rather than from this file.

The 2026-09-07 migration left very little of that to review. Measured on the new
repository the same day:

| | |
| --- | --- |
| Pull requests | 5, all from the migration itself |
| Issues | 0 |
| Releases | 0 — desktop releases live on the website repository |
| Tags | 33 |
| Branches | `main` only |
| Wiki | disabled |
| Actions runs | 3, all Dependabot dependency-graph updates, not workflow runs |

What does become public and is worth knowing: the author email
`minsiansu@gmail.com` on 1222 commits, and the 26 tracked files under `.claude/`
— agent-facing prose that becomes public documentation.

Also confirm the top of `README.md` states plainly that Apache-2.0 covers the
source and **not** the dictionary data, that attribution reaches the shipped
applications and not only the repository, and that the `SECURITY.md` address is
ready for the volume a public repository attracts.

## 9. Flip

Only after every section above is settled — not §4 alone. §4 is the one that is
outright blocked, but §7 has not been audited at all and §8 has not been
reviewed, and both are pre-publication work.

```sh
gh repo edit taigikeyboard/taigikeyboard --visibility public
```

## 10. Turn on GitHub's own scanning

Secret scanning and push protection come free with a public repository; on a
private one they need a paid GitHub security entitlement, which this repository
does not have — so in practice §9 is what makes them available here. Push
protection rejects a push containing a recognised credential before the objects
reach the server. It is the server-side preventive layer, the one gate a clone
cannot skip and `--no-verify` cannot reach, though a push can still be bypassed
by whoever holds that permission.

```sh
gh api --method PATCH repos/taigikeyboard/taigikeyboard \
  -f 'security_and_analysis[secret_scanning][status]=enabled' \
  -f 'security_and_analysis[secret_scanning_push_protection][status]=enabled' \
  -f 'security_and_analysis[secret_scanning_non_provider_patterns][status]=enabled'
```

Same default for anything created later in the organization:

```sh
gh api --method PATCH orgs/taigikeyboard \
  -F secret_scanning_push_protection_enabled_for_new_repositories=true
```

Then enable private vulnerability reporting, and verify:

```sh
gh api repos/taigikeyboard/taigikeyboard --jq '.visibility, .security_and_analysis'
```

---

## What runs in the meantime

None of this depends on the repository being public.

- `.githooks/pre-commit` scans staged changes and refuses the commit on a hit.
  It is **available, not automatic**: each clone activates it with `make hooks`,
  which sets `core.hooksPath`. A clone that never ran it has no local gate, and
  `git commit --no-verify` skips it in one that did.
- `make scan-secrets` scans everything `.gitleaks-scanned`'s watermark does not
  already cover. `make scan-secrets-full` rescans the whole history and prints
  the new baseline values to write into that file — it does not edit the file
  itself, so a re-baseline is always a reviewed commit.
- `.github/workflows/secrets.yml` runs on every pull request, every push to
  `main`, and Monday's schedule. Its triggers were commented out between
  2026-09-05 and 2026-09-07 because the run that opened PR #693 took 50 s, of
  which 42 s was `actions/checkout` with `fetch-depth: 0` dragging in dead
  `dictionary.db` blobs and 139 ms was the scan itself.

  They are back on because no scan in that workflow reads the whole history any
  more. A pull request and a push are scanned over their own commits; the weekly
  run and every fallback scan `--all --not <watermark>`. All of them cover a
  bounded set of commits, so the checkout can use `filter: blob:none` on every
  event: the commits and trees still arrive in full, so ranges and the watermark
  resolve, and the scan pulls only the blobs its own commits touch. (The checkout
  still materialises the working tree, so those blobs are fetched regardless.)

  What none of this reaches is a commit that no longer has a ref. A branch
  force-pushed away or deleted before Monday, with no pull request open, is
  unreachable from every surviving ref. A weekly full-history pass had the same
  hole; it too could only read what still existed.

  The workflow fails loudly rather than scanning less than it claims — if
  `.gitleaks-scanned` names a commit the repository does not have, or records a
  gitleaks version other than the one the run installed.

Two gates therefore fire on their own — the workflow, which no clone can skip,
and the local hook, which is opt-in per clone and skippable with `--no-verify`.

All three go through `scripts/gitleaks-scan.sh`, which is what keeps their
coverage rules and exit-code handling identical, and all three read
`.gitleaks.toml` and `.gitleaksignore` from the repository root.
When a finding is a false positive, add its fingerprint to `.gitleaksignore` with a
comment saying what the value actually is — never silence one you have not read.
