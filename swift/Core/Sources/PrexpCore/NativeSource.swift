import Darwin
import Foundation

/// The native macOS `ProcessSource` — talks straight to libproc/Mach via the
/// `Darwin` module. A Swift reimplementation of Rust's `MacosProcessSource`
/// (`prexp-core`) + `prexp-ffi`; there is no linkage between the two.
///
/// Every field's computation mirrors the Rust FFI exactly (units, Mach-timebase
/// conversion, page-size scaling, state codes) so both implementations produce
/// the same data.
public struct NativeSource: ProcessSource {
    public init() {}

    // MARK: Snapshots

    public func snapshotAll() throws -> [ProcessSnapshot] {
        let pids = try Self.listAllPids()
        var out = [ProcessSnapshot]()
        out.reserveCapacity(pids.count)
        for pid in pids {
            do {
                out.append(try snapshotPid(pid))
            } catch PrexpError.processNotFound {
                continue  // exited between list and query — skip
            } catch PrexpError.permissionDenied, PrexpError.backend {
                out.append(Self.partialSnapshot(pid))  // include a partial record
            }
        }
        return out
    }

    public func processSummaries() throws -> [ProcessSummary] {
        let pids = try Self.listAllPids()
        var out = [ProcessSummary]()
        out.reserveCapacity(pids.count)
        for pid in pids {
            // No fd enumeration — just the process-info call. Skip failures.
            if let info = try? Self.processInfo(pid) {
                out.append(ProcessSummary(pid: pid, name: info.name,
                                          cpuTimeNs: info.cpuTimeNs, memoryPhys: info.memoryPhys))
            }
        }
        return out
    }

    public func snapshotPid(_ pid: Int32) throws -> ProcessSnapshot {
        let info = try Self.processInfo(pid)
        let fds = Self.listFds(pid)
        let resources = fds.map { Self.resolve($0, pid: pid) }
        return ProcessSnapshot(
            pid: pid, ppid: info.ppid, name: info.name, state: info.state, accessible: true,
            memory: ProcessMemory(rss: info.memoryRss, phys: info.memoryPhys),
            activity: ProcessActivity(cpuTimeNs: info.cpuTimeNs, threadCount: info.threadCount,
                                      faults: info.faults, contextSwitches: info.contextSwitches,
                                      syscallsMach: info.syscallsMach, syscallsUnix: info.syscallsUnix),
            diskIo: DiskIo(bytesRead: info.diskBytesRead, bytesWritten: info.diskBytesWritten),
            resources: resources)
    }

    public func findByPath(_ path: String) throws -> [ProcessSnapshot] {
        let pids = Self.listPidsByPath(path)
        var out = [ProcessSnapshot]()
        for pid in pids {
            guard let snap = try? snapshotPid(pid) else { continue }
            // Only include if the process actually has this exact path open.
            if snap.resources.contains(where: { $0.path == path }) {
                out.append(snap)
            }
        }
        return out
    }

    public func processPath(_ pid: Int32) throws -> String {
        let path = Self.pidPath(pid)
        if path.isEmpty { throw PrexpError.backend("proc_pidpath(\(pid)) failed") }
        return path
    }

    // MARK: PID enumeration

    static func listAllPids() throws -> [Int32] {
        // NOTE: `proc_listallpids` returns the number of PIDs (a COUNT), not bytes
        // — unlike `proc_listpidspath` below, which returns bytes. Over-allocate 2×
        // for pid churn and truncate to the returned count (matches Rust).
        let count = proc_listallpids(nil, 0)
        guard count > 0 else { throw PrexpError.backend("proc_listallpids failed") }
        let capacity = Int(count) * 2
        var pids = [pid_t](repeating: 0, count: capacity)
        let actual = proc_listallpids(&pids, Int32(capacity * MemoryLayout<pid_t>.size))
        guard actual > 0 else { throw PrexpError.backend("proc_listallpids failed") }
        return Array(pids.prefix(Int(actual)))  // keep pid 0 (kernel) — Rust does too
    }

    static func listPidsByPath(_ path: String) -> [Int32] {
        // `proc_listpidspath` returns BYTES (not a count). Two-call sizing, 2×
        // over-allocation, then drop the zero padding.
        let flag = UInt32(PROC_ALL_PIDS)
        let byteSize = path.withCString { proc_listpidspath(flag, 0, $0, 0, nil, 0) }
        guard byteSize > 0 else { return [] }
        let capacity = (Int(byteSize) / MemoryLayout<pid_t>.size) * 2
        var pids = [pid_t](repeating: 0, count: capacity)
        let got = path.withCString { cpath in
            pids.withUnsafeMutableBytes { buf in
                proc_listpidspath(flag, 0, cpath, 0, buf.baseAddress, Int32(buf.count))
            }
        }
        guard got > 0 else { return [] }
        let n = Int(got) / MemoryLayout<pid_t>.size
        return Array(pids.prefix(n)).filter { $0 > 0 }
    }

    // MARK: Per-process info (proc_bsdinfo + proc_taskinfo + phys footprint + rusage)

    struct BasicInfo {
        var ppid: Int32
        var name: String
        var state: ProcessState
        var startedSecs: UInt64
        var uid: UInt32
        var nice: Int32
        var threadCount: Int32
        var memoryRss: UInt64
        var virtualSize: UInt64
        var cpuTimeNs: UInt64
        var faults: Int32
        var contextSwitches: Int32
        var syscallsMach: Int32
        var syscallsUnix: Int32
        var memoryPhys: UInt64
        var diskBytesRead: UInt64
        var diskBytesWritten: UInt64
    }

    static func processInfo(_ pid: Int32) throws -> BasicInfo {
        var bsd = proc_bsdinfo()
        let bsdSize = Int32(MemoryLayout<proc_bsdinfo>.size)
        let r = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &bsd, bsdSize)
        guard r == bsdSize else { throw errnoError(pid) }

        var name = cStringTuple(bsd.pbi_name)
        if name.isEmpty { name = cStringTuple(bsd.pbi_comm) }

        let task = taskInfo(pid)  // may be all-zero if it fails; matches graceful degrade
        let phys = physFootprint(pid)
        let (rd, wr) = diskIo(pid)

        return BasicInfo(
            ppid: Int32(bitPattern: bsd.pbi_ppid),
            name: name,
            state: processState(bsd.pbi_status),
            startedSecs: UInt64(bsd.pbi_start_tvsec),
            uid: bsd.pbi_uid,
            nice: bsd.pbi_nice,
            threadCount: Int32(task.pti_threadnum),
            memoryRss: task.pti_resident_size,
            virtualSize: task.pti_virtual_size,
            cpuTimeNs: MachTimebase.toNanos(task.pti_total_user &+ task.pti_total_system),
            faults: task.pti_faults,
            contextSwitches: task.pti_csw,
            syscallsMach: task.pti_syscalls_mach,
            syscallsUnix: task.pti_syscalls_unix,
            memoryPhys: phys,
            diskBytesRead: rd,
            diskBytesWritten: wr)
    }

    static func taskInfo(_ pid: Int32) -> proc_taskinfo {
        var ti = proc_taskinfo()
        let size = Int32(MemoryLayout<proc_taskinfo>.size)
        _ = proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &ti, size)
        return ti
    }

    /// Physical footprint via `task_name_for_pid` + `task_info(TASK_VM_INFO)`.
    static func physFootprint(_ pid: Int32) -> UInt64 {
        var task: task_name_t = 0
        guard task_name_for_pid(mach_task_self_, pid, &task) == KERN_SUCCESS else { return 0 }
        defer { mach_port_deallocate(mach_task_self_, task) }
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(task, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        return kr == KERN_SUCCESS ? UInt64(info.phys_footprint) : 0
    }

    /// Disk I/O via `proc_pid_rusage(RUSAGE_INFO_V4)`.
    static func diskIo(_ pid: Int32) -> (read: UInt64, written: UInt64) {
        var ru = rusage_info_v4()
        let rc = withUnsafeMutablePointer(to: &ru) {
            $0.withMemoryRebound(to: rusage_info_t?.self, capacity: 1) {
                proc_pid_rusage(pid, RUSAGE_INFO_V4, $0)
            }
        }
        guard rc == 0 else { return (0, 0) }
        return (ru.ri_diskio_bytesread, ru.ri_diskio_byteswritten)
    }

    static func processState(_ status: UInt32) -> ProcessState {
        switch status {
        case 1: return .idle       // SIDL
        case 2: return .running    // SRUN
        case 3: return .sleeping   // SSLEEP
        case 4: return .stopped    // SSTOP
        case 5: return .zombie     // SZOMB
        default: return .unknown
        }
    }

    /// Partial record for a process we couldn't fully inspect (permission denied).
    static func partialSnapshot(_ pid: Int32) -> ProcessSnapshot {
        if let info = try? processInfo(pid) {
            return ProcessSnapshot(
                pid: pid, ppid: info.ppid, name: info.name, state: info.state, accessible: false,
                memory: ProcessMemory(rss: info.memoryRss, phys: info.memoryPhys),
                activity: ProcessActivity(cpuTimeNs: info.cpuTimeNs, threadCount: info.threadCount,
                                          faults: info.faults, contextSwitches: info.contextSwitches,
                                          syscallsMach: info.syscallsMach, syscallsUnix: info.syscallsUnix),
                diskIo: DiskIo(bytesRead: info.diskBytesRead, bytesWritten: info.diskBytesWritten),
                resources: [])
        }
        let name = pidName(pid) ?? "pid:\(pid)"
        return ProcessSnapshot(
            pid: pid, ppid: 0, name: name, state: .unknown, accessible: false,
            memory: ProcessMemory(rss: 0, phys: 0),
            activity: ProcessActivity(cpuTimeNs: 0, threadCount: 0, faults: 0,
                                      contextSwitches: 0, syscallsMach: 0, syscallsUnix: 0),
            diskIo: DiskIo(bytesRead: 0, bytesWritten: 0), resources: [])
    }

    static func pidName(_ pid: Int32) -> String? {
        var buf = [CChar](repeating: 0, count: 2 * Int(MAXCOMLEN) + 1)
        let n = proc_name(pid, &buf, UInt32(buf.count))
        return n > 0 ? cStringBuffer(buf) : nil
    }

    static func pidPath(_ pid: Int32) -> String {
        var buf = [CChar](repeating: 0, count: Int(MAXPATHLEN))
        let n = proc_pidpath(pid, &buf, UInt32(buf.count))
        return n > 0 ? cStringBuffer(buf) : ""
    }

    // MARK: fd enumeration + classification

    struct Fd { var fd: Int32; var fdtype: UInt32 }

    static func listFds(_ pid: Int32) -> [Fd] {
        let fdSize = MemoryLayout<proc_fdinfo>.size
        let needed = proc_pidinfo(pid, PROC_PIDLISTFDS, 0, nil, 0)
        guard needed > 0 else { return [] }
        let allocSize = Int(needed) * 2  // over-allocate for fd churn (matches Rust)
        let count = allocSize / fdSize
        var buf = [proc_fdinfo](repeating: proc_fdinfo(), count: count)
        let got = proc_pidinfo(pid, PROC_PIDLISTFDS, 0, &buf, Int32(allocSize))
        guard got > 0 else { return [] }
        let n = Int(got) / fdSize
        return buf.prefix(n).map { Fd(fd: $0.proc_fd, fdtype: $0.proc_fdtype) }
    }

    static func resolve(_ fd: Fd, pid: Int32) -> OpenResource {
        switch Int32(bitPattern: fd.fdtype) {
        case PROX_FDTYPE_VNODE:
            if let path = vnodePath(pid: pid, fd: fd.fd) {
                let kind: ResourceKind = path.hasPrefix("/dev/") ? .device : .file
                return OpenResource(descriptor: fd.fd, kind: kind, path: path.isEmpty ? nil : path)
            }
            return OpenResource(descriptor: fd.fd, kind: .file, path: nil)
        case PROX_FDTYPE_SOCKET:
            return OpenResource(descriptor: fd.fd, kind: .socket, path: nil)
        case PROX_FDTYPE_PIPE:
            return OpenResource(descriptor: fd.fd, kind: .pipe, path: nil)
        case PROX_FDTYPE_KQUEUE:
            return OpenResource(descriptor: fd.fd, kind: .kqueue, path: nil)
        default:
            return OpenResource(descriptor: fd.fd, kind: .unknown, path: nil)
        }
    }

    static func vnodePath(pid: Int32, fd: Int32) -> String? {
        var info = vnode_fdinfowithpath()
        let size = Int32(MemoryLayout<vnode_fdinfowithpath>.size)
        let r = proc_pidfdinfo(pid, fd, PROC_PIDFDVNODEPATHINFO, &info, size)
        guard r == size else { return nil }
        return cStringTuple(info.pvip.vip_path)
    }

    // MARK: errno → PrexpError

    static func errnoError(_ pid: Int32) -> PrexpError {
        switch errno {
        case ESRCH: return .processNotFound(pid: pid)
        case EPERM, EACCES: return .permissionDenied(pid: pid)
        default: return .backend("syscall failed for pid \(pid) (errno \(errno))")
        }
    }
}
