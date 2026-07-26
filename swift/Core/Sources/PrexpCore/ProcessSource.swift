import Foundation

/// Errors surfaced by a `ProcessSource` — a Swift mirror of Rust `PrexpError`.
public enum PrexpError: Error, Sendable, Equatable {
    /// The process exited between enumeration and query.
    case processNotFound(pid: Int32)
    /// We lack permission to inspect this process.
    case permissionDenied(pid: Int32)
    /// A backend/syscall failure with a human-readable reason.
    case backend(String)
    /// The operation isn't supported on this platform.
    case unsupported(String)
}

/// Platform-agnostic contract for querying processes and system metrics — the
/// Swift analogue of Rust's `ProcessSource` trait. Implementations: `NativeSource`
/// (libproc/Mach) and test doubles with canned data. Default methods return
/// `.unsupported` so a partial double only overrides what it needs.
public protocol ProcessSource: Sendable {
    /// Snapshot all visible processes and their open file descriptors.
    func snapshotAll() throws -> [ProcessSnapshot]

    /// A lightweight summary of every visible process (no fd enumeration) — cheap
    /// enough to poll the whole table on an interval.
    func processSummaries() throws -> [ProcessSummary]

    /// Snapshot a single process by PID.
    func snapshotPid(_ pid: Int32) throws -> ProcessSnapshot

    /// The full path to a process's executable.
    func processPath(_ pid: Int32) throws -> String

    /// Send signal `sig` to `pid` (15 = SIGTERM, 9 = SIGKILL).
    func killProcess(_ pid: Int32, _ sig: Int32) throws

    /// Reverse lookup: processes that have `path` open.
    func findByPath(_ path: String) throws -> [ProcessSnapshot]

    /// Per-CPU tick counts for all cores.
    func cpuTicks() throws -> [CpuTicks]

    /// System memory information.
    func memoryInfo() throws -> MemoryInfo

    /// Cumulative system-wide network byte counters (rx/tx).
    func networkCounters() throws -> NetworkCounters

    /// Cumulative system-wide disk byte counters (read/written).
    func diskCounters() throws -> DiskCounters

    /// Each logical CPU's cluster type (P/E), aligned to `cpuTicks`. Static —
    /// read once and cache.
    func cpuPerfLevels() throws -> [CpuKind]

    /// System boot time in Unix epoch seconds.
    func systemBootTimeSecs() throws -> UInt64

    /// Load average (1, 5, 15 minute).
    func systemLoadAverage() throws -> (Double, Double, Double)

    /// The primary battery's state.
    func systemBattery() throws -> BatteryInfo

    /// Detailed process information for the info panel.
    func processDetail(_ pid: Int32, parentName: String) throws -> ProcessDetail
}

// Default implementations: most report `.unsupported`; `killProcess` is a plain
// POSIX syscall so it works everywhere.
public extension ProcessSource {
    func processSummaries() throws -> [ProcessSummary] { throw PrexpError.unsupported("process summaries") }
    func processPath(_ pid: Int32) throws -> String { throw PrexpError.unsupported("process path") }
    func networkCounters() throws -> NetworkCounters { throw PrexpError.unsupported("network counters") }
    func diskCounters() throws -> DiskCounters { throw PrexpError.unsupported("disk counters") }
    func cpuPerfLevels() throws -> [CpuKind] { throw PrexpError.unsupported("cpu perf levels") }
    func systemBootTimeSecs() throws -> UInt64 { throw PrexpError.unsupported("boot time") }
    func systemLoadAverage() throws -> (Double, Double, Double) { throw PrexpError.unsupported("load average") }
    func systemBattery() throws -> BatteryInfo { throw PrexpError.unsupported("battery") }
    func processDetail(_ pid: Int32, parentName: String) throws -> ProcessDetail { throw PrexpError.unsupported("process detail") }

    /// POSIX `kill(2)` — works on macOS and Linux alike. `sig` 0 tests existence.
    func killProcess(_ pid: Int32, _ sig: Int32) throws {
        if kill(pid, sig) != 0 {
            throw PrexpError.backend("kill(pid=\(pid), sig=\(sig)) failed")
        }
    }
}
