import Foundation

// System-level value types — a Swift mirror of `rust/crates/prexp-core/src/system.rs`.

/// Per-CPU tick counts (raw, cumulative — diff two reads for a rate).
public struct CpuTicks: Sendable, Equatable {
    public var user: UInt64
    public var system: UInt64
    public var idle: UInt64
    public var nice: UInt64

    public init(user: UInt64, system: UInt64, idle: UInt64, nice: UInt64) {
        self.user = user; self.system = system; self.idle = idle; self.nice = nice
    }

    /// Total ticks across all states.
    public var total: UInt64 { user &+ system &+ idle &+ nice }
    /// Busy ticks (everything but idle).
    public var busy: UInt64 { user &+ system &+ nice }
}

/// System memory information (bytes). `swap_*` are the swap file totals (`0` when none).
public struct MemoryInfo: Sendable, Equatable {
    public var total: UInt64
    public var used: UInt64
    public var free: UInt64
    public var wired: UInt64
    public var compressed: UInt64
    public var swapTotal: UInt64
    public var swapUsed: UInt64

    public init(total: UInt64, used: UInt64, free: UInt64, wired: UInt64,
                compressed: UInt64, swapTotal: UInt64, swapUsed: UInt64) {
        self.total = total; self.used = used; self.free = free; self.wired = wired
        self.compressed = compressed; self.swapTotal = swapTotal; self.swapUsed = swapUsed
    }
}

/// Cumulative system-wide network byte counters (monotonic; diff for a rate).
public struct NetworkCounters: Sendable, Equatable {
    public var rxBytes: UInt64
    public var txBytes: UInt64
    public init(rxBytes: UInt64, txBytes: UInt64) { self.rxBytes = rxBytes; self.txBytes = txBytes }
}

/// Cumulative system-wide disk byte counters (monotonic; diff for a rate).
public struct DiskCounters: Sendable, Equatable {
    public var readBytes: UInt64
    public var writeBytes: UInt64
    public init(readBytes: UInt64, writeBytes: UInt64) { self.readBytes = readBytes; self.writeBytes = writeBytes }
}

/// A logical CPU's cluster type. Apple Silicon splits cores into Performance and
/// Efficiency clusters; uniform hardware (Intel) reports `.unknown`.
public enum CpuKind: Sendable, Equatable {
    case performance
    case efficiency
    case unknown
}

/// The primary battery's state. Absent on a machine with no battery.
public struct BatteryInfo: Sendable, Equatable {
    /// Charge as a 0...100 percentage.
    public var percent: Double
    public var charging: Bool
    /// Minutes to empty while discharging, or `-1` when charging / calculating.
    public var timeToEmptyMin: Int32
    /// Minutes to full while charging, or `-1` otherwise.
    public var timeToFullMin: Int32

    public init(percent: Double, charging: Bool, timeToEmptyMin: Int32, timeToFullMin: Int32) {
        self.percent = percent; self.charging = charging
        self.timeToEmptyMin = timeToEmptyMin; self.timeToFullMin = timeToFullMin
    }
}
