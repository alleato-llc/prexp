import Foundation
import PrexpCore
import Tint

/// Which modal overlay (if any) is showing.
public enum OverlayKind: Sendable, Equatable {
    case none
    case detail   // selected process's open resources
    case help
}

/// The TUI's application state and behavior — the testable core of the front-end.
/// Holds an `any ProcessSource` (native in the binary, a canned double in tests),
/// pulls snapshots on `refresh`, and mutates state in `handle`. Rendering is a
/// separate pure function (`View.render`).
public final class AppModel: @unchecked Sendable {
    // @unchecked: the model is only ever touched on the main run loop (tint drives
    // render + onKey + the refresh timer there), so the mutable state is safe.
    public let source: any ProcessSource

    /// The current view: sorted + filtered processes.
    public private(set) var processes: [ProcessSnapshot] = []
    public private(set) var allProcesses: [ProcessSnapshot] = []
    public var selected: Int = 0
    public var sort: SortField = .cpu
    public var reversed: Bool = false
    /// nil = not in search mode; non-nil = the current query (possibly empty).
    public private(set) var search: String?
    public private(set) var overlay: OverlayKind = .none
    public var showStats: Bool = false
    public private(set) var error: String?
    public private(set) var quitRequested = false
    public let theme: PrexpTheme

    // system stats
    public private(set) var coreUsage: [Double] = []
    public private(set) var perfLevels: [CpuKind] = []
    public private(set) var memory: MemoryInfo?
    public private(set) var load: (Double, Double, Double) = (0, 0, 0)

    private let cpu = CpuTracker()
    private var prevTicks: [CpuTicks] = []

    public init(source: any ProcessSource, theme: PrexpTheme = .dark) {
        self.source = source
        self.theme = theme
        self.perfLevels = (try? source.cpuPerfLevels()) ?? []
    }

    public var isSearching: Bool { search != nil }
    public var searchQuery: String { search ?? "" }
    public var selectedProcess: ProcessSnapshot? {
        processes.indices.contains(selected) ? processes[selected] : nil
    }

    /// CPU% for a pid (from the delta tracker).
    public func cpuPercent(_ pid: Int32) -> Double { cpu.percent(pid) }

    // MARK: Data

    public func refresh(now: Date) {
        do {
            let snaps = try source.snapshotAll()
            cpu.update(snaps, now: now)
            allProcesses = snaps

            if let ticks = try? source.cpuTicks() {
                if prevTicks.count == ticks.count && !prevTicks.isEmpty {
                    coreUsage = zip(ticks, prevTicks).map { CoreUsage.percent(current: $0, previous: $1) }
                }
                prevTicks = ticks
            }
            memory = try? source.memoryInfo()
            load = (try? source.systemLoadAverage()) ?? (0, 0, 0)
            error = nil
            rebuild()
        } catch {
            self.error = "\(error)"
        }
    }

    /// Re-apply search + sort to `allProcesses`, preserving the selected PID.
    public func rebuild() {
        let selectedPid = selectedProcess?.pid
        var list = allProcesses
        if let search, !search.isEmpty { list = list.filter { matchesQuery($0, search) } }
        processes = sortProcesses(list, by: sort, reversed: reversed, cpu: cpu.percent)
        if let selectedPid, let i = processes.firstIndex(where: { $0.pid == selectedPid }) {
            selected = i
        }
        selected = min(max(0, selected), max(0, processes.count - 1))
    }

    // MARK: Input

    /// Handle a key. Text keys route to the search field while searching.
    public func handle(_ key: Key) {
        if search != nil {
            var q = search!
            if TextInput.edit(&q, key: key) { search = q; rebuild(); return }
            switch key {
            case .enter: search = q.isEmpty ? nil : q   // confirm; empty query exits search
            case .escape: search = nil; rebuild()
            default: break
            }
            if !isSearching { rebuild() }
            return
        }

        switch key {
        case .char("q"), .ctrlC:
            if overlay != .none { overlay = .none } else { quitRequested = true }
        case .escape:
            overlay = .none
        case .char("j"), .down: move(1)
        case .char("k"), .up: move(-1)
        case .home: selected = 0
        case .end: selected = max(0, processes.count - 1)
        case .char("s"): sort = sort.next(); reversed = false; rebuild()
        case .char("S"): reversed.toggle(); rebuild()
        case .char("/"): search = ""; overlay = .none
        case .char("g"): showStats.toggle()
        case .enter: overlay = (overlay == .detail) ? .none : .detail
        case .char("?"): overlay = (overlay == .help) ? .none : .help
        default: break
        }
    }

    private func move(_ delta: Int) {
        guard !processes.isEmpty else { return }
        selected = min(max(0, selected + delta), processes.count - 1)
    }
}
