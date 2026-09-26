// Opening the engine's user data in this process.

import Foundation

/// Opens the stores the engine keeps in the App Group container — once per
/// process: the app and its keyboard each open the same files — and keeps
/// them out of the OS backup (`docs/architecture/behavioral-invariants.md`
/// §29), the takeover's `<file>.pre-engine` copies included.
enum UserDataOpening {
    private static let lock = NSLock()
    private nonisolated(unsafe) static var opened = false

    /// Whether this process opened the user data. Picks are not reported
    /// otherwise: the engine would refuse each one (a keyboard without Full
    /// Access cannot write the container, and a test process never opens).
    static var isOpen: Bool {
        lock.withLock { opened }
    }

    /// Opens the user data in `directory` (the App Group container; `nil`
    /// leaves it closed), once per process — the keyboard's `viewDidLoad`
    /// runs again whenever the keyboard reappears. Answers at once: the engine
    /// finishes the takeover of the files the old repositories wrote on a
    /// thread of its own. The files are marked excluded from backup once that
    /// is done — every file and copy exists then, a pre-R7 one included.
    static func open(in directory: URL?) {
        guard let directory, !isOpen else { return }
        guard RustEngineBridge.userDataOpen(directory: directory, inBackground: true) else { return }
        lock.withLock { opened = true }
        DispatchQueue.global(qos: .utility).async {
            // The same open again, waiting this time: it answers once the
            // takeover is done. A GCD thread rather than a task — the wait can
            // last seconds and must not hold a thread of the shared pool.
            RustEngineBridge.userDataOpen(directory: directory, inBackground: false)
            excludeFromBackup(in: directory)
        }
    }

    /// Every store file and its pre-takeover copy that exists in `directory`,
    /// marked excluded from backup. Internal for the tests.
    static func excludeFromBackup(in directory: URL) {
        for name in RustEngineBridge.userDataFileNames {
            for file in [name, "\(name).pre-engine"] {
                let url = directory.appendingPathComponent(file)
                if FileManager.default.fileExists(atPath: url.path) {
                    url.excludeFromBackup()
                }
            }
        }
    }
}
