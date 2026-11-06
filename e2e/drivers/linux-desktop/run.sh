#!/usr/bin/env bash
# `make e2e PLATFORM=linux-desktop`: drive every scenario inside the REAL
# logged-in desktop session of each Linux dogfood VM (roadmap PR3c), with the
# test-mode build swapped into the session's own framework and swapped back
# afterwards (e2e/drivers/linux/desktop.py). Usage: run.sh <run-dir> [scenario-id]
#
#   linux-gnome-ibus  UTM arm64 Ubuntu, GNOME Wayland + IBus   (this Mac)
#   linux-kde-fcitx5  VirtualBox amd64 Ubuntu, KDE X11 + Fcitx5 (the `win` box)
#
# A powered-off VM is started and gets SSH_WAIT_S to come up; unreachable, no
# logged-in session, or a locked screen → that VM's scenarios are `skipped`.
# VM setup (autologin, screen lock off, python3-evdev, gnome-screenshot,
# passwordless sudo, build deps): memory reference_linux_dogfood_vms. Nothing
# may take keyboard focus from the host window, or every scenario times out
# waiting for it: apport off on Ubuntu (`enabled=0` in /etc/default/apport —
# its crash dialogs pop up over the session), and GNOME not starting in the
# overview (Ubuntu: dash-to-dock `disable-overview-on-startup` true).
set -euo pipefail

RUN_DIR="${1:?usage: run.sh <run-dir> [scenario-id]}"
ONLY="${2:-}"
SSH_WAIT_S=240
UTMCTL=/Applications/UTM.app/Contents/MacOS/utmctl
UTM_VM=Linux
VBOX_VM=taigi-linux
VBOXMANAGE="'C:\\Program Files\\Oracle\\VirtualBox\\VBoxManage.exe'"
source "$(dirname "${BASH_SOURCE[0]}")/../linux/remote.sh"

start_utm() { [ -x "$UTMCTL" ] && "$UTMCTL" start "$UTM_VM" >/dev/null 2>&1; }
start_vbox() { ssh -o ConnectTimeout=5 -o BatchMode=yes win "& $VBOXMANAGE startvm $VBOX_VM --type headless" >/dev/null 2>&1; }

# drive <platform> <framework> <ssh command> <host> <start command>
drive() {
    local platform=$1 framework=$2 ssh_cmd=$3 host=$4 start=$5
    local driver_cmd=(python3 "$E2E_REPO_ROOT/e2e/drivers/linux/desktop.py" --framework "$framework" --platform "$platform" --out "$RUN_DIR" ${ONLY:+--only "$ONLY"})
    if ! e2e_reachable "$ssh_cmd" "$host"; then
        echo "$platform: $host unreachable, starting the VM"
        if $start; then
            local deadline=$((SECONDS + SSH_WAIT_S))
            until e2e_reachable "$ssh_cmd" "$host" || [ $SECONDS -ge $deadline ]; do sleep 5; done
        fi
        if ! e2e_reachable "$ssh_cmd" "$host"; then
            "${driver_cmd[@]}" --skip "VM $host unreachable"
            return
        fi
    fi
    e2e_sync "$ssh_cmd" "$host"
    e2e_build "$ssh_cmd" "$host"
    # The driver restores in its own `finally`; this covers a killed ssh.
    trap "$ssh_cmd $host python3 $E2E_REMOTE/src/e2e/drivers/linux/desktop.py --restore || true" EXIT
    $ssh_cmd "$host" "cd $E2E_REMOTE && python3 src/e2e/drivers/linux/desktop.py --framework $framework --platform $platform \
        --prefix \$HOME/$E2E_REMOTE/prefix --out \$HOME/$E2E_REMOTE/run ${ONLY:+--only $ONLY}"
    trap - EXIT
    rsync -a -e "$ssh_cmd" "$host:$E2E_REMOTE/run/" "$RUN_DIR/"
}

mkdir -p "$RUN_DIR"
drive linux-gnome-ibus ibus ssh "${TAIGI_E2E_LINUX_HOST:-utm-linux}" start_utm
drive linux-kde-fcitx5 fcitx5 "ssh -J win -p 2222" taigi@127.0.0.1 start_vbox
