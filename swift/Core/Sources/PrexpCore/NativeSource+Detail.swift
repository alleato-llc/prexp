import Darwin
import Foundation

// Detailed process info for the info panel — mirrors `get_process_detail` in
// prexp-ffi's process.rs, including the per-connection network table.
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
            network: Self.networkConnections(pid),
            environment: Self.environment(pid))
    }

    // MARK: network connections — proc_pidfdinfo(PROC_PIDFDSOCKETINFO)

    /// TCP/UDP connections for a process — mirrors Rust `get_network_connections`
    /// + `resolve_socket_detail`. Ports and addresses are formatted exactly as the
    /// Rust side does (including that the port is used raw, without `ntohs` — a
    /// shared quirk; a byte-swapped 443 shows as 47873. Keep the two in lockstep).
    static func networkConnections(_ pid: Int32) -> [NetworkConnection] {
        var out: [NetworkConnection] = []
        for fd in listFds(pid) where Int32(bitPattern: fd.fdtype) == PROX_FDTYPE_SOCKET {
            if let conn = socketDetail(pid: pid, fd: fd.fd) { out.append(conn) }
        }
        return out
    }

    static func socketDetail(pid: Int32, fd: Int32) -> NetworkConnection? {
        var si = socket_fdinfo()
        let sz = Int32(MemoryLayout<socket_fdinfo>.size)
        guard proc_pidfdinfo(pid, fd, PROC_PIDFDSOCKETINFO, &si, sz) == sz else { return nil }

        let family = si.psi.soi_family
        let type = si.psi.soi_type  // SOCK_STREAM = 1 (tcp), SOCK_DGRAM = 2 (udp)

        // Only AF_INET (2) / AF_INET6 (30) are parsed; others are opaque.
        guard family == 2 || family == 30 else {
            return NetworkConnection(proto: "sock(\(family))", localAddr: "*", remoteAddr: nil, state: nil)
        }

        // in_sockinfo lives at the start of the soi_proto union (== tcpsi_ini for TCP).
        let ini = si.psi.soi_proto.pri_in
        let proto = type == 1 ? "tcp" : "udp"
        let local = formatSockAddr(ini.insi_laddr.ina_46.i46a_addr4.s_addr,
                                   port: ini.insi_lport, vflag: ini.insi_vflag)
        let remote = formatSockAddr(ini.insi_faddr.ina_46.i46a_addr4.s_addr,
                                    port: ini.insi_fport, vflag: ini.insi_vflag)
        let state = type == 1 ? tcpStateName(si.psi.soi_proto.pri_tcp.tcpsi_state) : nil
        let remoteOpt = ini.insi_fport != 0 ? remote : nil
        return NetworkConnection(proto: proto, localAddr: local, remoteAddr: remoteOpt, state: state)
    }

    static func formatSockAddr(_ sAddr: UInt32, port: Int32, vflag: UInt8) -> String {
        let ip: String
        if vflag & 0x1 != 0 {  // INI_IPV4
            let b = withUnsafeBytes(of: sAddr) { Array($0) }
            ip = (b[0] == 0 && b[1] == 0 && b[2] == 0 && b[3] == 0)
                ? "*" : "\(b[0]).\(b[1]).\(b[2]).\(b[3])"
        } else {
            ip = "*"  // IPv6 display simplified — matches Rust
        }
        return port == 0 ? "\(ip):*" : "\(ip):\(port)"
    }

    static func tcpStateName(_ s: Int32) -> String {
        switch s {
        case 0: return "CLOSED"
        case 1: return "LISTEN"
        case 2: return "SYN_SENT"
        case 3: return "SYN_RCVD"
        case 4: return "ESTABLISHED"
        case 5: return "CLOSE_WAIT"
        case 6: return "FIN_WAIT_1"
        case 7: return "CLOSING"
        case 8: return "LAST_ACK"
        case 9: return "FIN_WAIT_2"
        case 10: return "TIME_WAIT"
        default: return "UNKNOWN"
        }
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
