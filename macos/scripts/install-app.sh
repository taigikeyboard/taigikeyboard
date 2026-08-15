#!/usr/bin/env bash
# Deploy, remove, or restart the macOS input method in ~/Library/Input Methods
# (no sudo). Assumes `bundle-app.sh` has already assembled the bundle.
#
# Usage: install-app.sh <install|uninstall|reload>

set -euo pipefail

# shellcheck source=lib/bundle-identity.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/bundle-identity.sh"

LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister

# Terminate only processes whose executable path is exactly the installed one.
# `pkill -f <path>` would substring-match every command line — it hits unrelated
# processes (including the shell running this script, whose own arguments
# contain the pattern) and treats `.` as a wildcard. `ps -o comm=` reports the
# path recorded at exec time, so this still finds the process after the bundle
# has been moved or deleted.
stop_installed_instances() {
    ps -axo pid=,comm= | while read -r pid executable; do
        if [[ "$executable" == "$INSTALLED_EXECUTABLE" ]]; then
            echo "  stopping pid $pid"
            kill "$pid"
        fi
    done
}

install_app() {
    if [[ ! -d "$BUILT_APP" ]]; then
        echo "error: $BUILT_APP missing — run bundle-app.sh first" >&2
        exit 1
    fi

    # The old bundle moves aside before the processes are killed, deliberately:
    # while it is still at the install path, typing in any client relaunches it
    # the moment the process dies, and the relaunched copy would then be deleted
    # out from under a live process. Moving rather than deleting keeps a restore
    # path if the copy fails; the `.previous` suffix stops LaunchServices from
    # treating the set-aside directory as an app in the meantime.
    local previous="$INSTALLED_APP.previous"
    local is_first_install=true
    if [[ -d "$INSTALLED_APP" ]]; then
        is_first_install=false
        echo "==> Setting the installed copy aside"
        rm -rf "$previous"
        mv "$INSTALLED_APP" "$previous"
    fi

    echo "==> Stopping running instances"
    stop_installed_instances

    echo "==> Installing to $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"
    if ! cp -R "$BUILT_APP" "$INSTALL_DIR/"; then
        echo "error: install failed, restoring the previous copy" >&2
        rm -rf "$INSTALLED_APP"
        [[ -d "$previous" ]] && mv "$previous" "$INSTALLED_APP"
        exit 1
    fi
    rm -rf "$previous"

    "$LSREGISTER" -f -R -trusted "$INSTALLED_APP"
    # The staging copy carries the same bundle ID, so LaunchServices could
    # resolve to it and start the wrong process; AppDelegate's installed-copy
    # guard is the backstop, this keeps it from having to fire.
    "$LSREGISTER" -u "$BUILT_APP"

    echo "✓ Installed $INSTALLED_APP"
    if [[ "$is_first_install" == true ]]; then
        echo ""
        echo "ℹ First install. System Settings → Keyboard → Input Sources caches the"
        echo "  input-source list per login, so $APP_NAME may not appear until you"
        echo "  log out and back in."
    fi
}

uninstall_app() {
    if [[ ! -d "$INSTALLED_APP" ]]; then
        echo "Nothing to remove — $APP_NAME is not installed."
        return
    fi
    stop_installed_instances
    rm -rf "$INSTALLED_APP"
    echo "✓ Removed $INSTALLED_APP"
    echo "ℹ It stays listed in Input Sources until you log out and back in."
}

case "${1:-}" in
    install) install_app ;;
    uninstall) uninstall_app ;;
    reload)
        stop_installed_instances
        echo "✓ Restarted (the system relaunches it on the next keystroke)"
        ;;
    *)
        echo "usage: $(basename "$0") <install|uninstall|reload>" >&2
        exit 2
        ;;
esac
