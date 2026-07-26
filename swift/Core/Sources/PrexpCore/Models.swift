import Foundation

// Domain models — a Swift mirror of `rust/crates/prexp-core/src/models.rs`.
// Property order matches the Rust struct field order, and JSON keys match the
// Rust serde output (snake_case via the encoder's key strategy; enums serialize
// as their lowercase names). Kept in lockstep so both implementations emit the
// same JSON shape (the behavioral contract between the two ecosystems).

/// Process state. Serializes lowercase (`running`, `sleeping`, …).
public enum ProcessState: String, Codable, Sendable {
    case running, sleeping, stopped, zombie, idle, unknown

    /// Short label used by the UIs (mirrors Rust `ProcessState::label`).
    public var label: String {
        switch self {
        case .running: return "RUN"
        case .sleeping: return "SLP"
        case .stopped: return "STP"
        case .zombie: return "ZMB"
        case .idle: return "IDL"
        case .unknown: return "???"
        }
    }
}

/// The type of an open resource. Serializes lowercase (`file`, `socket`, …).
public enum ResourceKind: String, Codable, Sendable {
    case file, socket, pipe, device, kqueue, unknown
}

/// A network connection associated with a process.
public struct NetworkConnection: Codable, Sendable, Equatable {
    public var proto: String
    public var localAddr: String
    public var remoteAddr: String?
    public var state: String?

    public init(proto: String, localAddr: String, remoteAddr: String?, state: String?) {
        self.proto = proto
        self.localAddr = localAddr
        self.remoteAddr = remoteAddr
        self.state = state
    }

    enum CodingKeys: String, CodingKey {
        case proto
        case localAddr = "local_addr"
        case remoteAddr = "remote_addr"
        case state
    }
}

/// Memory usage for a process (bytes).
public struct ProcessMemory: Codable, Sendable, Equatable {
    /// Resident set size.
    public var rss: UInt64
    /// Physical footprint (private memory).
    public var phys: UInt64

    public init(rss: UInt64, phys: UInt64) {
        self.rss = rss
        self.phys = phys
    }
}

/// CPU and scheduling activity for a process.
public struct ProcessActivity: Codable, Sendable, Equatable {
    /// Cumulative CPU time (user + system) in nanoseconds.
    public var cpuTimeNs: UInt64
    public var threadCount: Int32
    public var faults: Int32
    public var contextSwitches: Int32
    public var syscallsMach: Int32
    public var syscallsUnix: Int32

    public init(cpuTimeNs: UInt64, threadCount: Int32, faults: Int32,
                contextSwitches: Int32, syscallsMach: Int32, syscallsUnix: Int32) {
        self.cpuTimeNs = cpuTimeNs
        self.threadCount = threadCount
        self.faults = faults
        self.contextSwitches = contextSwitches
        self.syscallsMach = syscallsMach
        self.syscallsUnix = syscallsUnix
    }

    enum CodingKeys: String, CodingKey {
        case cpuTimeNs = "cpu_time_ns"
        case threadCount = "thread_count"
        case faults
        case contextSwitches = "context_switches"
        case syscallsMach = "syscalls_mach"
        case syscallsUnix = "syscalls_unix"
    }
}

/// Disk I/O counters for a process (bytes).
public struct DiskIo: Codable, Sendable, Equatable {
    public var bytesRead: UInt64
    public var bytesWritten: UInt64

    public init(bytesRead: UInt64, bytesWritten: UInt64) {
        self.bytesRead = bytesRead
        self.bytesWritten = bytesWritten
    }

    enum CodingKeys: String, CodingKey {
        case bytesRead = "bytes_read"
        case bytesWritten = "bytes_written"
    }
}

/// A single open file descriptor or resource.
public struct OpenResource: Codable, Sendable, Equatable {
    public var descriptor: Int32
    public var kind: ResourceKind
    public var path: String?

    public init(descriptor: Int32, kind: ResourceKind, path: String?) {
        self.descriptor = descriptor
        self.kind = kind
        self.path = path
    }

    // Encode `path` even when nil (serde emits `"path": null`; Swift's synthesized
    // encoder would omit the key). Explicit encode keeps JSON parity with Rust.
    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(descriptor, forKey: .descriptor)
        try c.encode(kind, forKey: .kind)
        try c.encode(path, forKey: .path)  // encodes null when nil
    }

    enum CodingKeys: String, CodingKey {
        case descriptor, kind, path
    }
}

/// A snapshot of a single process and its open resources.
public struct ProcessSnapshot: Codable, Sendable, Equatable {
    public var pid: Int32
    public var ppid: Int32
    public var name: String
    public var state: ProcessState
    /// Whether we had full access to this process's fds. `false` when permission
    /// was denied — pid/name are still valid.
    public var accessible: Bool
    public var memory: ProcessMemory
    public var activity: ProcessActivity
    public var diskIo: DiskIo
    public var resources: [OpenResource]

    public init(pid: Int32, ppid: Int32, name: String, state: ProcessState,
                accessible: Bool, memory: ProcessMemory, activity: ProcessActivity,
                diskIo: DiskIo, resources: [OpenResource]) {
        self.pid = pid
        self.ppid = ppid
        self.name = name
        self.state = state
        self.accessible = accessible
        self.memory = memory
        self.activity = activity
        self.diskIo = diskIo
        self.resources = resources
    }

    /// Count resources by kind (mirrors Rust `ProcessSnapshot::count_by_kind`).
    public func count(of kind: ResourceKind) -> Int {
        resources.lazy.filter { $0.kind == kind }.count
    }

    enum CodingKeys: String, CodingKey {
        case pid, ppid, name, state, accessible, memory, activity
        case diskIo = "disk_io"
        case resources
    }
}

/// A lightweight per-process summary (no fd enumeration), cheap enough to poll
/// the whole table on an interval.
public struct ProcessSummary: Codable, Sendable, Equatable {
    public var pid: Int32
    public var name: String
    /// Cumulative CPU time (user + system) in nanoseconds.
    public var cpuTimeNs: UInt64
    /// Physical footprint (private memory) in bytes.
    public var memoryPhys: UInt64

    public init(pid: Int32, name: String, cpuTimeNs: UInt64, memoryPhys: UInt64) {
        self.pid = pid
        self.name = name
        self.cpuTimeNs = cpuTimeNs
        self.memoryPhys = memoryPhys
    }

    enum CodingKeys: String, CodingKey {
        case pid, name
        case cpuTimeNs = "cpu_time_ns"
        case memoryPhys = "memory_phys"
    }
}

/// Detailed process information for the info panel. Mirrors Rust `ProcessDetail`.
public struct ProcessDetail: Codable, Sendable, Equatable {
    public var pid: Int32
    public var ppid: Int32
    public var parentName: String
    public var name: String
    public var path: String
    public var cwd: String
    public var user: String
    public var uid: UInt32
    public var state: ProcessState
    public var nice: Int32
    public var startedSecs: UInt64
    public var threadCount: Int32
    public var virtualSize: UInt64
    public var memoryRss: UInt64
    public var memoryPhys: UInt64
    public var cpuTimeNs: UInt64
    public var fdFiles: Int
    public var fdSockets: Int
    public var fdPipes: Int
    public var fdOther: Int
    public var fdTotal: Int
    public var faults: Int32
    public var contextSwitches: Int32
    public var syscallsMach: Int32
    public var syscallsUnix: Int32
    public var diskBytesRead: UInt64
    public var diskBytesWritten: UInt64
    public var network: [NetworkConnection]
    /// KEY/VALUE environment pairs.
    public var environment: [EnvVar]

    public init(pid: Int32, ppid: Int32, parentName: String, name: String, path: String,
                cwd: String, user: String, uid: UInt32, state: ProcessState, nice: Int32,
                startedSecs: UInt64, threadCount: Int32, virtualSize: UInt64, memoryRss: UInt64,
                memoryPhys: UInt64, cpuTimeNs: UInt64, fdFiles: Int, fdSockets: Int, fdPipes: Int,
                fdOther: Int, fdTotal: Int, faults: Int32, contextSwitches: Int32,
                syscallsMach: Int32, syscallsUnix: Int32, diskBytesRead: UInt64,
                diskBytesWritten: UInt64, network: [NetworkConnection], environment: [EnvVar]) {
        self.pid = pid; self.ppid = ppid; self.parentName = parentName; self.name = name
        self.path = path; self.cwd = cwd; self.user = user; self.uid = uid; self.state = state
        self.nice = nice; self.startedSecs = startedSecs; self.threadCount = threadCount
        self.virtualSize = virtualSize; self.memoryRss = memoryRss; self.memoryPhys = memoryPhys
        self.cpuTimeNs = cpuTimeNs; self.fdFiles = fdFiles; self.fdSockets = fdSockets
        self.fdPipes = fdPipes; self.fdOther = fdOther; self.fdTotal = fdTotal; self.faults = faults
        self.contextSwitches = contextSwitches; self.syscallsMach = syscallsMach
        self.syscallsUnix = syscallsUnix; self.diskBytesRead = diskBytesRead
        self.diskBytesWritten = diskBytesWritten; self.network = network; self.environment = environment
    }

    enum CodingKeys: String, CodingKey {
        case pid, ppid
        case parentName = "parent_name"
        case name, path, cwd, user, uid, state, nice
        case startedSecs = "started_secs"
        case threadCount = "thread_count"
        case virtualSize = "virtual_size"
        case memoryRss = "memory_rss"
        case memoryPhys = "memory_phys"
        case cpuTimeNs = "cpu_time_ns"
        case fdFiles = "fd_files"
        case fdSockets = "fd_sockets"
        case fdPipes = "fd_pipes"
        case fdOther = "fd_other"
        case fdTotal = "fd_total"
        case faults
        case contextSwitches = "context_switches"
        case syscallsMach = "syscalls_mach"
        case syscallsUnix = "syscalls_unix"
        case diskBytesRead = "disk_bytes_read"
        case diskBytesWritten = "disk_bytes_written"
        case network, environment
    }
}

/// One environment entry — Rust represents these as a `(String, String)` tuple,
/// which serde encodes as a 2-element JSON array `["KEY", "VALUE"]`.
public struct EnvVar: Sendable, Equatable {
    public var key: String
    public var value: String

    public init(_ key: String, _ value: String) {
        self.key = key
        self.value = value
    }
}

extension EnvVar: Codable {
    public init(from decoder: Decoder) throws {
        var c = try decoder.unkeyedContainer()
        key = try c.decode(String.self)
        value = try c.decode(String.self)
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.unkeyedContainer()
        try c.encode(key)
        try c.encode(value)
    }
}
