import Foundation
import PrexpCore

/// Per-process CPU% via the delta of cumulative `cpu_time_ns` between refreshes,
/// divided by wall-clock elapsed — mirrors the Rust TUI's `stats.rs`. The first
/// update establishes a baseline (everything reads 0%); subsequent updates yield
/// real percentages. `now` is injected so the logic is deterministic in tests.
public final class CpuTracker {
    private var prev: [Int32: UInt64] = [:]
    private var prevAt: Date?
    private var pct: [Int32: Double] = [:]

    public init() {}

    public func update(_ snapshots: [ProcessSnapshot], now: Date) {
        if let prevAt {
            let elapsed = now.timeIntervalSince(prevAt)
            if elapsed > 0 {
                let elapsedNs = elapsed * 1_000_000_000
                var next = [Int32: Double]()
                for s in snapshots {
                    let cur = s.activity.cpuTimeNs
                    if let old = prev[s.pid], cur >= old {
                        next[s.pid] = Double(cur - old) / elapsedNs * 100.0
                    }
                }
                pct = next
            }
        }
        prev = Dictionary(snapshots.map { ($0.pid, $0.activity.cpuTimeNs) },
                          uniquingKeysWith: { a, _ in a })
        prevAt = now
    }

    public func percent(_ pid: Int32) -> Double { pct[pid] ?? 0 }
}

/// Per-core usage percentage from two `CpuTicks` reads (busy delta / total delta).
public enum CoreUsage {
    public static func percent(current: CpuTicks, previous: CpuTicks) -> Double {
        let totalDelta = Double(current.total &- previous.total)
        guard totalDelta > 0 else { return 0 }
        let busyDelta = Double(current.busy &- previous.busy)
        return min(max(busyDelta / totalDelta * 100.0, 0), 100)
    }
}
