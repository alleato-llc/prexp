import Darwin
import Foundation

// Detailed process info for the info panel — mirrors `get_process_detail` in
// prexp-ffi's process.rs.
//
// NOTE: `network` (per-connection tcp/udp table) is not parsed here yet. It reads
// the `socket_fdinfo` proto union, whose exact address/port formatting must match
// the Rust `resolve_socket_detail` byte-for-byte — best verified against the Rust
// info panel, so it lands with the Network tab in the front-end milestones.
extension NativeSource {

    public func processDetail(_ pid: Int32, parentName: String) throws -> ProcessDetail {
        var bsd = proc_bsdinfo()
        let bsdSize = Int32(MemoryLayout<proc_bsdinfo>.size)
        guard proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &bsd, bsdSize) == bsdSize else {
            throw Self.errnoError(pid)
        }
        let task = Self.taskInfo(pid)

        var name = cStringTuple(bsd.pbi_name)
        if name.isEmpty { name = cStringTuple(bsd.pbi_comm) }

        // fd counts
        var files = 0, sockets = 0, pipes = 0, other = 0
        let fds = Self.listFds(pid)
        for fd in fds {
            switch Int32(bitPattern: fd.fdtype) {
            case PROX_FDTYPE_VNODE: files += 1
            case PROX_FDTYPE_SOCKET: sockets += 1
            case PROX_FDTYPE_PIPE: pipes += 1
            default: other += 1
            }
        }
        let (rd, wr) = Self.diskIo(pid)

        return ProcessDetail(
            pid: pid,
            ppid: Int32(bitPattern: bsd.pbi_ppid),
            parentName: parentName,
            name: name,
            path: Self.pidPath(pid),
            cwd: Self.pidCwd(pid),
            user: Self.username(bsd.pbi_uid),
            uid: bsd.pbi_uid,
            state: Self.processState(bsd.pbi_status),
            nice: bsd.pbi_nice,
            startedSecs: UInt64(bsd.pbi_start_tvsec),
            threadCount: Int32(task.pti_threadnum),
            virtualSize: task.pti_virtual_size,
            memoryRss: task.pti_resident_size,
            memoryPhys: Self.physFootprint(pid),
            cpuTimeNs: MachTimebase.toNanos(task.pti_total_user &+ task.pti_total_system),
            fdFiles: files, fdSockets: sockets, fdPipes: pipes, fdOther: other, fdTotal: fds.count,
            faults: task.pti_faults,
            contextSwitches: task.pti_csw,
            syscallsMach: task.pti_syscalls_mach,
            syscallsUnix: task.pti_syscalls_unix,
            diskBytesRead: rd, diskBytesWritten: wr,
            network: [],
            environment: Self.environment(pid))
    }

    // MARK: cwd — proc_pidinfo(PROC_PIDVNODEPATHINFO)

    static func pidCwd(_ pid: Int32) -> String {
        var info = proc_vnodepathinfo()
        let size = Int32(MemoryLayout<proc_vnodepathinfo>.size)
        guard proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, 0, &info, size) == size else { return "" }
        return cStringTuple(info.pvi_cdir.vip_path)
    }

    // MARK: username — getpwuid

    static func username(_ uid: UInt32) -> String {
        if let pw = getpwuid(uid), let name = pw.pointee.pw_name {
            return String(cString: name)
        }
        return String(uid)
    }

    // MARK: environment — sysctl KERN_PROCARGS2 parse

    /// Extract `KEY=VALUE` environment pairs from the process's argument area.
    /// Layout: `[argc:i32][exec_path\0][padding\0…][argv\0…][env\0…\0]`. Returns
    /// empty on any failure (never throws) — mirrors the Rust behavior.
    static func environment(_ pid: Int32) -> [EnvVar] {
        let KERN_PROCARGS2: Int32 = 49
        var mib: [Int32] = [CTL_KERN, KERN_PROCARGS2, pid]

        var size = 0
        guard Darwin.sysctl(&mib, 3, nil, &size, nil, 0) == 0, size > 0 else { return [] }
        var buffer = [UInt8](repeating: 0, count: size)
        guard Darwin.sysctl(&mib, 3, &buffer, &size, nil, 0) == 0, size >= 4 else { return [] }
        buffer.removeLast(buffer.count - size)

        // argc: first 4 bytes, native endian.
        let argc = buffer.prefix(4).withUnsafeBytes { $0.load(as: Int32.self) }
        var pos = 4
        let n = buffer.count

        func skipToNul() {
            while pos < n && buffer[pos] != 0 { pos += 1 }
            if pos < n { pos += 1 }  // step over the NUL
        }

        skipToNul()                                   // exec path
        while pos < n && buffer[pos] == 0 { pos += 1 } // padding
        for _ in 0..<max(argc, 0) { skipToNul() }      // argv[0..argc]

        var env = [EnvVar]()
        while pos < n {
            let start = pos
            while pos < n && buffer[pos] != 0 { pos += 1 }
            if pos == start { break }  // empty string terminates the env block
            let s = String(decoding: buffer[start..<pos], as: UTF8.self)
            if pos < n { pos += 1 }
            if let eq = s.firstIndex(of: "=") {
                env.append(EnvVar(String(s[..<eq]), String(s[s.index(after: eq)...])))
            }
        }
        return env
    }
}
