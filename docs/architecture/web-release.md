# Web Release

> **Type**: Release flow
> **Keywords**: `web`, `GitHub Pages`, `Cloudflare`, `deploy`
> **Status**: Active. Plan and decisions: `web-roadmap.md`.

The site lives at `https://taigikeyboard.tw/web/`, a project page of the `taigikeyboard/web` repository, which holds only the workflow below and no built files. The web version has no version number: it is a beta that follows `main`, and the build hash in `assets-manifest.json` is its identity.

## Deploy

`.github/workflows/web.yml` deploys hourly at minute 17 (drawn at random, after GitHub dropped most runs at the crowded minute 0 and the site went 38 hours without a deploy; GitHub may still start a scheduled run late or skip it): it runs its gates on `main`'s head and, when they pass, starts a deploy, which builds `main`'s head again (a commit pushed between the two goes out ungated). An hour whose head is the one the last successful scheduled run had deploys nothing. A push to `main` runs the gates only. A manual run (Actions → Web → Run workflow, on `main`) runs the gates and, when they pass, deploys `main`'s head whether or not it moved. CI runs Playwright in Chromium only; `make web-e2e` on a Mac runs all three engines.

The CI job builds and stages the site on the host, then runs the Chromium tests in the official `mcr.microsoft.com/playwright:v<version>-noble` image. The version comes from the installed `@playwright/test` package, pinned by `web/package-lock.json`. Browsers and their system libraries are already in that image, so the browser gate does not run apt against Ubuntu mirrors. Local `make web-e2e` still installs browsers on the host.

The edge keeps each old file for up to 600 seconds, so for those minutes a first visit can pair a new `pkg/taigi_web.js` with an old wasm and fail until a reload. Maintainer 2026-11-07: accepted for the beta, with no purge and no Cloudflare token; revisit if it hurts. So the deploy waits 11 minutes, then runs `check-live` against the build it staged. A failed check leaves the run red in `taigikeyboard/web`, not here, and the next hourly deploy replaces the site.

To deploy by hand, prefer the gated manual run of `web.yml` above. `taigikeyboard/web` → Actions → Deploy → Run workflow deploys ungated. The deploy takes no input and always builds `main`'s head (maintainer 2026-11-10); there is no rollback to an older commit — fix forward on `main`.

Secret:

| Where | Name | What |
|---|---|---|
| this repository, environment `web-deploy` (deploys from `main` only, no reviewer) | `WEB_DEPLOY_TOKEN` | Fine-grained token, `taigikeyboard/web` only, Actions: write. It expires: a deploy job that fails on `gh workflow run` usually means this |

Create `taigikeyboard/web` and the token before this workflow reaches `main`, or every deploy job fails.

## Cloudflare

- The zone's catch-all cache rule for `taigikeyboard.tw` honours the origin's `max-age=600`. Dictionary names are content-hashed, so they have a rule of their own after it, because the later rule wins: `starts_with(http.request.uri.path, "/web/dictionaries/")`, edge TTL 30 days.
- Email Obfuscation is on for the zone and rewrites HTML, so no email address in a staged page.

## Workflow

`.github/workflows/deploy.yml` in `taigikeyboard/web`. Keep the tool steps in line with `.github/workflows/web.yml`.

```yaml
name: Deploy

on:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    outputs:
      build: ${{ steps.name.outputs.build }}
      commit: ${{ steps.commit.outputs.commit }}
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: taigikeyboard/taigikeyboard
          ref: main
          persist-credentials: false

      # Read once so the check job reads the same commit, not a newer main.
      - name: Name the commit
        id: commit
        run: |
          commit=$(git rev-parse HEAD)
          echo "commit=$commit" >> "$GITHUB_OUTPUT"
          echo "commit $commit" >> "$GITHUB_STEP_SUMMARY"

      - uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0
        with:
          node-version: '22'

      - uses: jdx/mise-action@f5be966e4a917d54244728e6a5fdcd1d93a35e6c # v5.2.6
        with:
          version: 2026.11.2
          install_args: protoc github:wasm-bindgen/wasm-bindgen github:WebAssembly/binaryen uv pipx:fonttools zig

      - name: Install the toolchain (web/rust-toolchain.toml)
        working-directory: web
        run: rustup toolchain install --no-self-update

      - name: Stage the site
        run: make -C web site

      - name: Name the build
        id: name
        run: |
          build=$(jq -r .build web/target/site/assets-manifest.json)
          echo "build=$build" >> "$GITHUB_OUTPUT"
          echo "build $build" >> "$GITHUB_STEP_SUMMARY"

      - uses: actions/upload-pages-artifact@fc324d3547104276b827a68afc52ff2a11cc49c9 # v5.0.0
        with:
          path: web/target/site

  deploy:
    needs: build
    runs-on: ubuntu-24.04
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@368f82528645a54fb793d4d04e342629a3f51346 # v5.0.1
        # A healthy deploy takes ~10 s; a stuck one never leaves an empty status.
        with:
          timeout: 120000

  check:
    needs: [build, deploy]
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: taigikeyboard/taigikeyboard
          ref: ${{ needs.build.outputs.commit }}
          persist-credentials: false
          sparse-checkout: web/scripts

      - uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0
        with:
          node-version: '22'

      # Longer than the edge's max-age=600.
      - name: Wait for the edge
        run: sleep 660

      - name: Check the live files
        run: node web/scripts/check-live.mjs https://taigikeyboard.tw/web/ "$BUILD"
        env:
          BUILD: ${{ needs.build.outputs.build }}
```
