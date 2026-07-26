/// Process sort fields — cycled with `s` (and direction toggled with `S`),
/// mirroring the Rust front-ends. Numeric fields default to descending (biggest
/// first); name/pid default to ascending.
public enum SortField: CaseIterable, Sendable {
    case cpu, memory, pmem, threads, files, total, name, pid

    public var label: String {
        switch self {
        case .cpu: return "CPU%"
        case .memory: return "MEM"
        case .pmem: return "PMEM"
        case .threads: return "THR"
        case .files: return "FILES"
        case .total: return "TOTAL"
        case .name: return "NAME"
        case .pid: return "PID"
        }
    }

    /// Whether this field's natural order is descending.
    var descendingByDefault: Bool {
        switch self {
        case .name, .pid: return false
        default: return true
        }
    }

    public func next() -> SortField {
        let all = SortField.allCases
        let i = all.firstIndex(of: self)!
        return all[(i + 1) % all.count]
    }
}

/// Sort a snapshot list. `cpu` supplies the (tracker-derived) CPU% since it isn't
/// on the snapshot itself. Stable, and `reversed` flips the natural direction.
public func sortProcesses(_ procs: [ProcessSnapshot], by field: SortField,
                          reversed: Bool, cpu: (Int32) -> Double) -> [ProcessSnapshot] {
    let sorted: [ProcessSnapshot]
    switch field {
    case .cpu:
        sorted = procs.sorted { cpu($0.pid) > cpu($1.pid) }
    case .memory:
        sorted = procs.sorted { $0.memory.rss > $1.memory.rss }
    case .pmem:
        sorted = procs.sorted { $0.memory.phys > $1.memory.phys }
    case .threads:
        sorted = procs.sorted { $0.activity.threadCount > $1.activity.threadCount }
    case .files:
        sorted = procs.sorted { $0.count(of: .file) > $1.count(of: .file) }
    case .total:
        sorted = procs.sorted { $0.resources.count > $1.resources.count }
    case .name:
        sorted = procs.sorted { $0.name.lowercased() < $1.name.lowercased() }
    case .pid:
        sorted = procs.sorted { $0.pid < $1.pid }
    }
    // `descendingByDefault` fields are already in descending order above; reversed
    // flips whatever the natural order is.
    return reversed ? sorted.reversed() : sorted
}

/// Case-insensitive substring match on name or PID — the `/` search predicate.
public func matchesQuery(_ p: ProcessSnapshot, _ query: String) -> Bool {
    if query.isEmpty { return true }
    let q = query.lowercased()
    return p.name.lowercased().contains(q) || String(p.pid).contains(q)
}
