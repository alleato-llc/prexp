import Foundation
import PrexpCore

/// A canned `ProcessSource` for driving the TUI model in tests — no live system.
struct FakeSource: ProcessSource {
    var procs: [ProcessSnapshot]

    func snapshotAll() throws -> [ProcessSnapshot] { procs }

    func snapshotPid(_ pid: Int32) throws -> ProcessSnapshot {
        guard let p = procs.first(where: { $0.pid == pid }) else {
            throw PrexpError.processNotFound(pid: pid)
        }
        return p
    }

    func findByPath(_ path: String) throws -> [ProcessSnapshot] {
        procs.filter { $0.resources.contains { $0.path == path } }
    }

    func cpuTicks() throws -> [CpuTicks] {
        [CpuTicks(user: 10, system: 5, idle: 85, nice: 0),
         CpuTicks(user: 20, system: 10, idle: 70, nice: 0)]
    }

    func memoryInfo() throws -> MemoryInfo {
        MemoryInfo(total: 16_000_000_000, used: 8_000_000_000, free: 8_000_000_000,
                   wired: 2_000_000_000, compressed: 1_000_000_000, swapTotal: 0, swapUsed: 0)
    }

    // Never signal a real process from tests.
    func killProcess(_ pid: Int32, _ sig: Int32) throws {}
}

enum Sample {
    static func proc(_ pid: Int32, _ name: String, rss: UInt64 = 1024, phys: UInt64 = 512,
                     threads: Int32 = 1, cpuNs: UInt64 = 0, accessible: Bool = true,
                     resources: [OpenResource] = []) -> ProcessSnapshot {
        ProcessSnapshot(
            pid: pid, ppid: 1, name: name, state: .running, accessible: accessible,
            memory: ProcessMemory(rss: rss, phys: phys),
            activity: ProcessActivity(cpuTimeNs: cpuNs, threadCount: threads, faults: 0,
                                      contextSwitches: 0, syscallsMach: 0, syscallsUnix: 0),
            diskIo: DiskIo(bytesRead: 0, bytesWritten: 0),
            resources: resources)
    }

    static let procs: [ProcessSnapshot] = [
        proc(200, "alpha", rss: 2_000_000, phys: 1_000_000, threads: 4, cpuNs: 5_000_000_000,
             resources: [OpenResource(descriptor: 3, kind: .file, path: "/tmp/a.log"),
                         OpenResource(descriptor: 4, kind: .socket, path: nil)]),
        proc(201, "bravo", rss: 500_000, phys: 250_000, threads: 2,
             resources: [OpenResource(descriptor: 5, kind: .file, path: "/etc/hosts")]),
        proc(202, "charlie", accessible: false),
    ]
}
