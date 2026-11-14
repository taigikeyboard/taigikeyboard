#!/usr/bin/env bash
# The monthly desktop train's steps, run by `.github/workflows/desktop-train.yml`
# (docs/architecture/release-automation-roadmap.md, Phase 5).
#
# Usage: desktop-train.sh plan           decide stop / cut / recut; for a cut, make
#                                        the release commit and push it to a
#                                        scratch branch
#        desktop-train.sh draft          start this version's draft afresh on the
#                                        checked-out commit
#        desktop-train.sh check-assets   fail unless every platform is on the draft
#        desktop-train.sh notify <text>  post to Discord (DISCORD_PR_WEBHOOK)
#
# `plan` holds no release secret: it runs `cargo update` (inside set-versions)
# and pushes only a scratch branch, with GITHUB_TOKEN. The App token that moves
# `main` lives in the workflow's `cut` job, behind the `release` environment's
# reviewer (roadmap D5).
#
# Environment: GH_TOKEN; plan also reads RELEASE_HOLD (the repository
# variable, D4), DRY_RUN (true | false), GITHUB_RUN_ID / GITHUB_RUN_ATTEMPT,
# DISCORD_PR_WEBHOOK. Outputs go to $GITHUB_OUTPUT, notes for the person who
# approves the cut to $GITHUB_STEP_SUMMARY (stdout when run by hand).

set -euo pipefail

fail() {
    echo "error: $*" >&2
    exit 1
}

REPOSITORY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

output() { echo "$1=$2" >> "${GITHUB_OUTPUT:-/dev/stdout}"; }
summary() { printf '%s\n' "$@" >> "${GITHUB_STEP_SUMMARY:-/dev/stdout}"; }

# The repository-level webhook: posting needs no environment, so neither an
# empty month nor a failure waits for an approval to be said. No mentions, no
# link previews — the same payload beta-release.sh posts.
notify() {
    [[ -n "${DISCORD_PR_WEBHOOK:-}" ]] || { echo "warning: DISCORD_PR_WEBHOOK is not set — not posting: $1" >&2; return 0; }
    python3 -c 'import json,sys; print(json.dumps({"content": sys.argv[1], "allowed_mentions": {"parse": []}, "flags": 4}))' "$1" |
        curl -sSf --retry 3 -H 'Content-Type: application/json' --data-binary @- "$DISCORD_PR_WEBHOOK" > /dev/null ||
        fail "cannot post to Discord"
}

# The version this run cuts: the month it is in Taiwan, where the schedule's
# 09:00 on the 1st is.
train_version() { TZ=Asia/Taipei date +%Y.%-m.0; }

plan() {
    if [[ -n "${RELEASE_HOLD:-}" ]]; then
        summary "RELEASE_HOLD is set (\"$RELEASE_HOLD\") — no cut. \`gh variable delete RELEASE_HOLD\` resumes the train."
        output action stop
        return
    fi

    local version="$SHORT_VERSION" head
    head="$(git -C "$REPOSITORY_DIR" rev-parse HEAD)"
    output version "$version"
    # Stage (move main, draft, sign, beta) only on a real run from main.
    local stage=true
    [[ "${DRY_RUN:-false}" != true ]] || stage=false
    output stage "$stage"

    # The cut's release commit is already on main: re-stage the draft from
    # main's tip (a fix landed after the cut, or a failed run is retried).
    if python3 "$REPOSITORY_DIR/tools/release_notes.py" check-versions --train desktop --version "$version" > /dev/null 2>&1; then
        local state
        state="$(desktop_release_state)" || exit 1
        [[ "$state" != published ]] ||
            fail "$DESKTOP_TAG is already published — a fix now is a patch: make version-desktop ${version%.0}.1, then /release-desktop <platform>"
        summary "The tree is already at $version: re-staging the $DESKTOP_TAG draft from ${head:0:7}."
        output action recut
        output source_sha "$head"
        return
    fi

    # Empty month: the generated notes — the whole release body (D2) — list no
    # pull request.
    local notes previous
    notes="$(mktemp)"
    previous="$(previous_desktop_tag)" || exit 1
    generated_release_notes "$head" "$previous" > "$notes"
    if ! grep -qE "/pull/[0-9]+" "$notes"; then
        local message="Desktop train: no pull request merged since $previous — no $version this month."
        summary "$message"
        output action stop
        if [[ "$stage" != true ]]; then
            echo "dry run — would post: $message"
        else
            notify "$message"
        fi
        return
    fi

    # The release commit. set-versions refuses a downgrade, so a mid-month
    # dispatch after a patch (tree YYYY.M.P) stops here.
    python3 "$REPOSITORY_DIR/tools/release_notes.py" set-versions --train desktop --version "$version"
    python3 "$REPOSITORY_DIR/tools/release_notes.py" check-versions --train desktop --version "$version"
    [[ -z "$(git -C "$REPOSITORY_DIR" ls-files --others --exclude-standard)" ]] ||
        fail "set-versions left untracked files — the release commit would not be what was built"
    git -C "$REPOSITORY_DIR" \
        -c user.name='github-actions[bot]' \
        -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
        commit --quiet --all --message "chore(release): desktop $version"
    local commit branch
    commit="$(git -C "$REPOSITORY_DIR" rev-parse HEAD)"

    # Hosted builds check the commit out by sha, so it has to be on the remote
    # before main moves (and without main moving, on a dry run). The `finish`
    # job deletes the branch.
    # The token goes through the repository config, not argv, and is removed
    # straight after the push.
    branch="desktop-train/${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
    git -C "$REPOSITORY_DIR" config --local http.https://github.com/.extraheader \
        "AUTHORIZATION: basic $(printf 'x-access-token:%s' "$GH_TOKEN" | base64 | tr -d '\n')"
    git -C "$REPOSITORY_DIR" push --quiet origin "HEAD:refs/heads/$branch" || {
        git -C "$REPOSITORY_DIR" config --local --unset-all http.https://github.com/.extraheader
        fail "cannot push $branch"
    }
    git -C "$REPOSITORY_DIR" config --local --unset-all http.https://github.com/.extraheader
    output scratch_branch "$branch"
    output action cut
    output source_sha "$commit"

    summary "### Cut $version" "" \
        "Approving the \`cut\` job fast-forwards \`main\` from ${head:0:7} to this one commit:" \
        "https://github.com/$RELEASE_REPOSITORY/compare/$head...$commit" "" \
        "<details><summary>Release notes (generated)</summary>" "" "$(cat "$notes")" "" "</details>"
}

draft() {
    reset_desktop_draft
    local release_id
    release_id="$(gh release view "$DESKTOP_TAG" --repo "$RELEASE_REPOSITORY" --json databaseId --jq .databaseId)"
    [[ -n "$release_id" ]] || fail "cannot read the draft $DESKTOP_TAG's id back"
    output release_id "$release_id"
    # An earlier month's draft — held, never published — is not removed here;
    # it is the maintainer's to delete.
    local stale
    stale="$(gh release list --repo "$RELEASE_REPOSITORY" --limit 100 --json tagName,isDraft \
        --jq "[.[] | select(.isDraft and (.tagName | startswith(\"desktop-\")) and .tagName != \"$DESKTOP_TAG\") | .tagName] | join(\" \")")"
    [[ -z "$stale" ]] || summary "Older unpublished desktop drafts: $stale — delete them once this one supersedes them."
}

check_assets() {
    require_staged_assets macos windows linux chromeos
}

# The release object's names: `plan` names this month's version; the other
# steps read the version from the checkout (the release commit).
SHORT_VERSION=""
[[ "${1:-}" != plan ]] || SHORT_VERSION="$(train_version)"
# shellcheck source=lib/desktop-release.sh
source "$REPOSITORY_DIR/tools/release/lib/desktop-release.sh"

case "${1:-}:$#" in
    plan:1) plan ;;
    draft:1) draft ;;
    check-assets:1) check_assets ;;
    notify:2) notify "$2" ;;
    *)
        echo "usage: desktop-train.sh plan | draft | check-assets | notify <text>" >&2
        exit 2
        ;;
esac
