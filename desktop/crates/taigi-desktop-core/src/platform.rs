//! Which desktop a shell is — a value the shell passes in, never read off the
//! build target: the rules of every desktop must be testable on any host
//! (`docs/architecture/macos-desktop-core-roadmap.md` D6).

/// The desktop input method a caller is. Each shell passes its own constant
/// (`taigi_windows_platform::DESKTOP_PLATFORM`,
/// `taigi_linux_platform::DESKTOP_PLATFORM`).
///
/// It selects, each beside the rule it parametrises:
/// - the user-data journal (`engine/user_data.rs`);
/// - the caret-chord modifier (`keys/intent.rs`);
/// - the chord grammar — modifier letters, reserved keys, the case fold of
///   a chord key, how a chord reads on screen (`keys/chord.rs`,
///   `keys/shortcut_labels.rs`).
///
/// Windows and Linux share every rule; MacOS carries the macOS input
/// method's own (`macos/Sources/TaigiInputMethodCore`). The engine itself is
/// never told which desktop is calling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DesktopPlatform {
    Windows,
    Linux,
    MacOS,
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::DesktopPlatform;

    /// The desktop a test pins when the platform is not what it tests — the
    /// rules that differ are pinned per desktop beside each rule.
    pub(crate) const TEST_PLATFORM: DesktopPlatform = DesktopPlatform::Windows;

    /// The two desktops that share every key rule — each tested on its own
    /// value, so a later split between them cannot pass unnoticed.
    pub(crate) const WINDOWS_AND_LINUX: [DesktopPlatform; 2] =
        [DesktopPlatform::Windows, DesktopPlatform::Linux];

    pub(crate) const ALL_PLATFORMS: [DesktopPlatform; 3] = [
        DesktopPlatform::Windows,
        DesktopPlatform::Linux,
        DesktopPlatform::MacOS,
    ];
}
