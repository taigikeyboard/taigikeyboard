# Sourced by tools/e2e/linux*/run.sh: put this tree on a Linux VM and build
# the test-mode IME there into ~/$E2E_REMOTE/prefix (the VM's own install is
# never touched). Each function takes the ssh command (one word-split string,
# e.g. "ssh -J win -p 2222") and the host.

E2E_REMOTE=taigi-e2e
E2E_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

e2e_reachable() {
    $1 -o ConnectTimeout=5 -o BatchMode=yes "$2" true 2>/dev/null
}

e2e_sync() {
    local ssh_cmd=$1 host=$2
    # Only what `make -C linux install` reads (the `../` inputs of
    # linux/Makefile, licence texts included) plus the harness. `target/`
    # stays on the VM so the next run builds incrementally.
    local trees=(engine desktop linux dictionaries i18n fonts symbols tools e2e)
    local files=(LICENSE NOTICE THIRD_PARTY_LICENSES.md dictionary/LICENSE)
    $ssh_cmd "$host" "mkdir -p $E2E_REMOTE/src/dictionary"
    # One rsync per tree rather than one `--relative` call: macOS openrsync
    # ignores --delete under --relative, and a file removed here (a retired
    # scenario, a moved source) must leave the VM too.
    local tree
    for tree in "${trees[@]}"; do
        (cd "$E2E_REPO_ROOT" && rsync -a -e "$ssh_cmd" --delete --exclude 'target/' --exclude '.git' --exclude '/runs/' \
            "$tree/" "$host:$E2E_REMOTE/src/$tree/")
    done
    (cd "$E2E_REPO_ROOT" && rsync -a -e "$ssh_cmd" --relative "${files[@]}" "$host:$E2E_REMOTE/src/")
}

e2e_build() {
    $1 "$2" bash -s <<EOF
set -euo pipefail
source "\$HOME/.cargo/env"
cd "\$HOME/$E2E_REMOTE"
rm -rf run prefix
make -s -C src/linux install E2E=1 PREFIX="\$HOME/$E2E_REMOTE/prefix" > build.log 2>&1 \
    || { tail -30 build.log; exit 1; }
EOF
}
