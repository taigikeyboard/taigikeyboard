import Foundation

/// Re-entry guard for the TPS ↔ layout 1:1 sync in `SharedSettings`.
///
/// When the user changes `inputMode` to `.tps`, the setter also flips
/// `keyboardLayoutType` to `.tps` — and vice versa. Without a guard,
/// that cross-write triggers infinite recursion. Each setter wraps its
/// own sync block in `sync { ... }`, so only the outer call runs and
/// the inner re-entry short-circuits.
final class TPSSyncCoordinator {
    private var isSyncing = false

    /// Run `block` once. If already inside a `sync` invocation (i.e. the
    /// current property setter was triggered by another setter), no-op.
    func sync(_ block: () -> Void) {
        guard !isSyncing else { return }
        isSyncing = true
        defer { isSyncing = false }
        block()
    }
}
