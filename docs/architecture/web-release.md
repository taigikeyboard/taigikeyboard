# Web Release

> **Type**: Release flow
> **Keywords**: `web`, `GitHub Pages`, `Cloudflare`, `deploy`
> **Status**: Active. Plan and decisions: `web-roadmap.md`.

The site lives at `https://taigikeyboard.tw/web/`, a project page of the `taigikeyboard/web` repository, which holds only the workflow below and no built files. The web version has no version number: it is a beta that follows `main`, and the build hash in `assets-manifest.json` is its identity.

## Deploy

Every push to `main` that `.github/workflows/web.yml` runs on and passes starts a deploy of that commit, so a change to the docs alone deploys nothing. CI runs Playwright in Chromium only; `make web-e2e` on a Mac runs all three engines.

The deploy waits for Pages, purges `/web/` from the edge, which otherwise holds each old file for up to 600 seconds so that a new `pkg/taigi_web.js` can meet an old wasm, then runs `check-live` against the build it staged, three times at most. A failed check leaves the run red in `taigikeyboard/web`, not here, and the next push replaces the site.

To deploy by hand, or to go back to a commit: `taigikeyboard/web` → Actions → Deploy → Run workflow, with the full commit.

Secrets:

| Where | Name | What |
|---|---|---|
| this repository | `WEB_DEPLOY_TOKEN` | Fine-grained token, `taigikeyboard/web` only, Actions: write. It expires: a deploy job that fails on `gh workflow run` usually means this |
| `taigikeyboard/web` | `CLOUDFLARE_PURGE_TOKEN` | API token, this zone, Cache Purge only |
| `taigikeyboard/web` | variable `CLOUDFLARE_ZONE_ID` | The zone |

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
    outputs:
      build: ${{ steps.name.outputs.build }}
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

      # `ref` takes any commit the repository can reach, a fork's included.
      - name: Refuse a commit that is not on main
        run: |
          status=$(gh api "repos/taigikeyboard/taigikeyboard/compare/main...$COMMIT" --jq .status)
          test "$status" = identical || test "$status" = behind
        env:
          GH_TOKEN: ${{ github.token }}
          COMMIT: ${{ inputs.commit }}

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

  check:
    needs: [build, deploy]
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: taigikeyboard/taigikeyboard
          ref: ${{ inputs.commit }}
          persist-credentials: false
          sparse-checkout: web/scripts

      - uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0
        with:
          node-version: '22'

      # A purge before Pages' own CDN has the new files caches the old ones again, and
      # the origin cannot be asked directly: github.io redirects to this domain.
      - name: Purge the edge and check the live files
        run: |
          for attempt in 1 2 3; do
            sleep 60
            curl --fail-with-body --silent --show-error -X POST \
              "https://api.cloudflare.com/client/v4/zones/$ZONE/purge_cache" \
              -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
              --data '{"prefixes":["taigikeyboard.tw/web/"]}'
            if node web/scripts/check-live.mjs https://taigikeyboard.tw/web/ "$BUILD"; then
              exit 0
            fi
          done
          exit 1
        env:
          ZONE: ${{ vars.CLOUDFLARE_ZONE_ID }}
          TOKEN: ${{ secrets.CLOUDFLARE_PURGE_TOKEN }}
          BUILD: ${{ needs.build.outputs.build }}
```
