# Web Release

> **Type**: Release flow
> **Keywords**: `web`, `GitHub Pages`, `Cloudflare`, `deploy`
> **Status**: Active. Plan and decisions: `web-roadmap.md`.

The site lives at `https://taigikeyboard.tw/web/`, a project page of the `taigikeyboard/web` repository, which holds only the workflow below. No token, and no built files in git.

## Deploy

By hand, from the Mac, in Taiwan:

1. `taigikeyboard/web` → Actions → Deploy → Run workflow, with the full commit of this repository. The run's summary names the build.
2. Purge the edge, which otherwise holds each old file for up to 600 seconds, so that a new `pkg/taigi_web.js` can meet an old wasm:
   `cf cache purge -z taigikeyboard.tw -f --body '{"prefixes":["taigikeyboard.tw/web/"]}'` (from a scratch directory: `cf` leaves a `.cloudflare/` behind).
3. `make -C web check-live`. It must name the build from step 1 and exit 0.

## Cloudflare

- The zone's catch-all cache rule for `taigikeyboard.tw` honours the origin's `max-age=600`. Dictionary names are content-hashed, so they have a rule of their own after it, because the later rule wins: `starts_with(http.request.uri.path, "/web/dictionaries/")`, edge TTL 30 days.
- Email Obfuscation is on for the zone and rewrites HTML, so no email address in a staged page.

## Workflow

`.github/workflows/deploy.yml` in `taigikeyboard/web`. Keep the tool steps in line with `.github/workflows/web.yml`.

```yaml
name: Deploy

on:
  workflow_dispatch:
    inputs:
      commit:
        description: Full commit of taigikeyboard/taigikeyboard
        required: true

permissions:
  contents: read

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: taigikeyboard/taigikeyboard
          ref: ${{ inputs.commit }}
          persist-credentials: false

      - uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0
        with:
          node-version: '22'

      - uses: jdx/mise-action@f5be966e4a917d54244728e6a5fdcd1d93a35e6c # v5.2.6
        with:
          version: 2026.11.2
          install_args: protoc github:wasm-bindgen/wasm-bindgen github:WebAssembly/binaryen uv pipx:fonttools

      - name: Install the toolchain (web/rust-toolchain.toml)
        working-directory: web
        run: rustup toolchain install --no-self-update

      - name: Stage the site
        run: make -C web site

      - name: Name the build
        run: jq -r '"build \(.build)"' web/target/site/assets-manifest.json >> "$GITHUB_STEP_SUMMARY"

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
```
