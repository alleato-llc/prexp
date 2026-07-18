import Darwin
import Foundation
import IOKit.ps

// System metrics — mirrors the `get_cpu_ticks` / `get_memory_info` / battery /
// load / boot / perf-level functions in `prexp-ffi`'s system.rs.
extension NativeSource {

    // MARK: CPU ticks — host_processor_info(PROCESSOR_CPU_LOAD_INFO)

    public func cpuTicks() throws -> [CpuTicks] {
        var count: natural_t = 0
        var info: processor_info_array_t?
        var infoCount: mach_msg_type_number_t = 0
        let kr = host_processor_info(mach_host_self(), PROCESSOR_CPU_LOAD_INFO, &count, &info, &infoCount)
        guard kr == KERN_SUCCESS, let info else {
            throw PrexpError.backend("host_processor_info failed (\(kr))")
        }
        defer {
            vm_deallocate(mach_task_self_, vm_address_t(bitPattern: info),
                          vm_size_t(infoCount) * vm_size_t(MemoryLayout<integer_t>.size))
        }
        let stride = Int(CPU_STATE_MAX)  // 4: user, system, idle, nice
        var out = [CpuTicks]()
        out.reserveCapacity(Int(count))
        for i in 0..<Int(count) {
            let base = i * stride
            out.append(CpuTicks(
                user: UInt64(UInt32(bitPattern: info[base + Int(CPU_STATE_USER)])),
                system: UInt64(UInt32(bitPattern: info[base + Int(CPU_STATE_SYSTEM)])),
                idle: UInt64(UInt32(bitPattern: info[base + Int(CPU_STATE_IDLE)])),
                nice: UInt64(UInt32(bitPattern: info[base + Int(CPU_STATE_NICE)]))))
        }
        return out
    }

    // MARK: Memory — sysctl + host_statistics64(HOST_VM_INFO64)

    public func memoryInfo() throws -> MemoryInfo {
        let total: UInt64 = Self.sysctl("hw.memsize") ?? 0
        let pageSize: UInt64 = Self.sysctl("hw.pagesize") ?? 4096

        var vmStat = vm_statistics64_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<vm_statistics64_data_t>.size / MemoryLayout<integer_t>.size)
        let kr = withUnsafeMutablePointer(to: &vmStat) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count)
            }
        }
        guard kr == KERN_SUCCESS else { throw PrexpError.backend("host_statistics64 failed (\(kr))") }

        let wired = UInt64(vmStat.wire_count) &* pageSize
        let compressed = UInt64(vmStat.compressor_page_count) &* pageSize
        let active = UInt64(vmStat.active_count) &* pageSize
        let used = active &+ wired &+ compressed
        let free = total > used ? total - used : 0

        let (swapTotal, swapUsed) = Self.swapUsage()
        return MemoryInfo(total: total, used: used, free: free, wired: wired,
                          compressed: compressed, swapTotal: swapTotal, swapUsed: swapUsed)
    }

    static func swapUsage() -> (total: UInt64, used: UInt64) {
        var xsw = xsw_usage()
        var len = MemoryLayout<xsw_usage>.size
        guard sysctlbyname("vm.swapusage", &xsw, &len, nil, 0) == 0 else { return (0, 0) }
        return (xsw.xsu_total, xsw.xsu_used)
    }

    // MARK: Perf levels (P/E) — sysctl hw.perflevelN.logicalcpu

    public func cpuPerfLevels() throws -> [CpuKind] {
        let coreCount = Int(Self.sysctl("hw.logicalcpu") ?? 0)
        let levels = Int(Self.sysctl("hw.nperflevels") ?? 1)
        guard levels >= 2 else {
            // Uniform hardware (Intel) — no P/E split.
            return Array(repeating: .unknown, count: coreCount)
        }
        // perflevel0 = Performance cores, perflevel1 = Efficiency cores. On Apple
        // Silicon logical CPUs are ordered [P.., E..], aligning with cpuTicks order.
        let pCount = Int(Self.sysctl("hw.perflevel0.logicalcpu") ?? 0)
        let eCount = Int(Self.sysctl("hw.perflevel1.logicalcpu") ?? 0)
        var out = [CpuKind]()
        out.append(contentsOf: Array(repeating: .performance, count: pCount))
        out.append(contentsOf: Array(repeating: .efficiency, count: eCount))
        if out.count < coreCount {
            out.append(contentsOf: Array(repeating: .unknown, count: coreCount - out.count))
        }
        return out
    }

    // MARK: Load / boot

    public func systemLoadAverage() throws -> (Double, Double, Double) {
        var loads = [Double](repeating: 0, count: 3)
        guard getloadavg(&loads, 3) == 3 else { throw PrexpError.backend("getloadavg failed") }
        return (loads[0], loads[1], loads[2])
    }

    public func systemBootTimeSecs() throws -> UInt64 {
        var tv = timeval()
        var len = MemoryLayout<timeval>.size
        var mib: [Int32] = [CTL_KERN, KERN_BOOTTIME]
        guard Darwin.sysctl(&mib, 2, &tv, &len, nil, 0) == 0, tv.tv_sec > 0 else {
            throw PrexpError.backend("kern.boottime failed")
        }
        return UInt64(tv.tv_sec)
    }

    // MARK: Battery — IOKit power sources

    public func systemBattery() throws -> BatteryInfo {
        guard let blob = IOPSCopyPowerSourcesInfo()?.takeRetainedValue(),
              let list = IOPSCopyPowerSourcesList(blob)?.takeRetainedValue() as? [CFTypeRef] else {
            throw PrexpError.backend("no power sources")
        }
        for ps in list {
            guard let desc = IOPSGetPowerSourceDescription(blob, ps)?.takeUnretainedValue()
                    as? [String: Any] else { continue }
            guard let maxCap = desc[kIOPSMaxCapacityKey] as? Int, maxCap > 0,
                  let curCap = desc[kIOPSCurrentCapacityKey] as? Int else { continue }
            let percent = min(max(Double(curCap) * 100.0 / Double(maxCap), 0), 100)
            let charging = (desc[kIOPSIsChargingKey] as? Bool) ?? false
            let toEmpty = Int32((desc[kIOPSTimeToEmptyKey] as? Int) ?? -1)
            let toFull = Int32((desc[kIOPSTimeToFullChargeKey] as? Int) ?? -1)
            return BatteryInfo(percent: percent, charging: charging,
                               timeToEmptyMin: toEmpty, timeToFullMin: toFull)
        }
        throw PrexpError.backend("no valid power source")
    }

    // MARK: sysctl helper

    /// Read a scalar sysctl by name into `T` (integers). Returns nil on failure.
    static func sysctl<T: FixedWidthInteger>(_ name: String) -> T? {
        var value: T = 0
        var len = MemoryLayout<T>.size
        guard sysctlbyname(name, &value, &len, nil, 0) == 0 else { return nil }
        return value
    }
}
