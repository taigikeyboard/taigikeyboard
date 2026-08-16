// Opens a learning database off the main thread and serializes every access to it.

import Foundation

/// The queue, the connection and the "is it usable yet" answer that both
/// learning stores need, in one place.
///
/// Writes are asynchronous and best-effort. A frequency or association write is
/// a side effect of a keystroke that has already been rendered — making the
/// user wait on an `fsync` for it would turn a durable-write hiccup into a
/// visible input stall, and losing the last write to a crash costs one
/// increment of a counter.
///
/// Reads are synchronous, because the one read there is has to answer inside
/// the keystroke that asked: the candidate ranking for the composition on
/// screen cannot be filled in a frame later.
///
/// `@unchecked Sendable` covers `connection` and `isOpen`: the connection is
/// only ever touched from `queue`, and `isOpen` is guarded by its own lock
/// because callers on the main actor read it without waiting for the queue.
final class LearningDatabase: @unchecked Sendable {
    private let connection = SQLiteConnection()
    private let queue: DispatchQueue
    private let fileName: String
    private let directory: @Sendable () throws -> URL
    private let schema: @Sendable (SQLiteConnection) throws -> Void
    private let logger: DebugLogger

    private let stateLock = NSLock()
    private var isOpen = false
    private var hasStartedOpening = false

    /// - Parameters:
    ///   - fileName: the database file inside `directory`; created if absent.
    ///   - name: used for the queue label and the log category, so a failure
    ///     names the store it came from.
    ///   - directory: resolved on the queue at open time rather than at
    ///     construction, so a home directory that cannot be reached is one more
    ///     open failure instead of a second failure mode every caller has to
    ///     carry an optional for.
    ///   - schema: run once, on the queue, immediately after the file opens.
    init(
        fileName: String,
        name: String,
        directory: @escaping @Sendable () throws -> URL,
        schema: @escaping @Sendable (SQLiteConnection) throws -> Void,
    ) {
        // `.userInitiated` rather than `.utility`, because the read side of this
        // queue is on the keystroke path: the candidate ranking for the
        // composition on screen waits on it. A background-priority queue with a
        // durable write already in flight would make the main actor wait behind
        // it, which the user feels as the keyboard stalling.
        queue = DispatchQueue(label: "com.siansiansu.inputmethod.\(name)", qos: .userInitiated)
        self.fileName = fileName
        self.directory = directory
        self.schema = schema
        logger = DebugLogger(category: name)
    }

    /// True once the file is open and the schema has been applied. False both
    /// before that finishes and after it fails — callers treat the two the same
    /// way, by skipping the learned data for this keystroke.
    var isReady: Bool {
        stateLock.withLock { isOpen }
    }

    /// Opens the file and applies the schema, on the queue. Called once at
    /// launch; a failure is logged and leaves the store permanently not-ready
    /// rather than retrying, because the failures that reach here (no write
    /// permission, corrupt file) do not resolve themselves between keystrokes.
    func openInBackground() {
        let alreadyStarted = stateLock.withLock {
            defer { hasStartedOpening = true }
            return hasStartedOpening
        }
        guard !alreadyStarted else { return }
        queue.async { [self] in
            do {
                try connection.open(at: directory().appendingPathComponent(fileName))
                try schema(connection)
                stateLock.withLock { isOpen = true }
                logger.info("opened")
            } catch {
                logger.error("open failed: \(error)")
            }
        }
    }

    /// Queues a write. Silently does nothing until the store is ready, which is
    /// the same degrade as a write that fails: one unlearned commit.
    func write(_ body: @escaping @Sendable (SQLiteConnection) throws -> Void) {
        queue.async { [self] in
            guard stateLock.withLock({ isOpen }) else { return }
            do {
                try body(connection)
            } catch {
                logger.error("write failed: \(error)")
            }
        }
    }

    /// Runs a read on the queue and waits for it. `nil` means the store was not
    /// ready or the read threw — never an empty result, so a caller can tell
    /// "nothing learned yet" from "could not look".
    func read<Result>(_ body: (SQLiteConnection) throws -> Result) -> Result? {
        guard isReady else { return nil }
        return queue.sync {
            do {
                return try body(connection)
            } catch {
                logger.error("read failed: \(error)")
                return nil
            }
        }
    }
}
