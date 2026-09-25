#!/usr/bin/env bash
# Build + e2e dependencies for one distribution's container, as root
# (.github/workflows/linux-e2e.yml). Usage: ci-deps.sh ubuntu|debian|fedora|arch
#
# The linux/ build (Rust links GTK 4 / libadwaita, the Fcitx5 addon needs its
# headers), then what driver.py runs: Xvfb, xdotool, a GTK 3 entry through
# python gi, both frameworks with their GTK 3 input modules, a D-Bus session.
set -euo pipefail

debian_family() {
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates curl git unzip build-essential \
        ibus ibus-gtk3 dbus-daemon dbus-x11 xvfb xauth xdotool \
        python3 python3-gi gir1.2-gtk-3.0 fcitx5 fcitx5-frontend-gtk3 \
        libgtk-4-dev libadwaita-1-dev pkg-config \
        fcitx5-modules-dev extra-cmake-modules cmake ninja-build
}

case "${1:?usage: ci-deps.sh ubuntu|debian|fedora|arch}" in
    ubuntu | debian) debian_family ;;
    fedora)
        dnf install -y --setopt=install_weak_deps=False \
            git gcc gcc-c++ make cmake ninja-build extra-cmake-modules fcitx5-devel \
            gtk4-devel libadwaita-devel pkgconf-pkg-config unzip findutils which \
            fcitx5 fcitx5-gtk3 ibus ibus-gtk3 dbus-daemon dbus-tools \
            xorg-x11-server-Xvfb xorg-x11-xauth xdotool python3 python3-gobject gtk3
        ;;
    arch)
        pacman -Syu --noconfirm --needed \
            base-devel git cmake ninja extra-cmake-modules fcitx5 fcitx5-gtk ibus \
            gtk3 gtk4 libadwaita pkgconf unzip rustup \
            dbus xorg-server-xvfb xorg-xauth xdotool python python-gobject
        ;;
    *)
        echo "ci-deps.sh: unknown distribution '$1'" >&2
        exit 2
        ;;
esac
