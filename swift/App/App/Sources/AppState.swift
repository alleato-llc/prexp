import Foundation
import Observation
import PrexpCore

/// The GUI's observable state. `@MainActor` — all mutation happens on the main
/// actor; only the heavy `ProcessSource` calls hop to a detached task. Mirrors
/// the Rust `prexp-desktop` App: delta CPU metrics, a rolling CPU-history buffer,
/// a timer-driven refresh at the chosen interval.
@MainActor
@Observable
final class AppState {
    /// A display row: a snapshot plus its (tracker-derived) CPU%.
    struct Row: Identifiable, Sendable {
        let snapshot: ProcessSnapshot
        let cpu: Double
        var id: Int32 { snapshot.pid }
    }

    let source: any ProcessSource

    var rows: [Row] = []
    var selection: Int32?
    var sort: SortField = .cpu { didSet { rebuild() } }
    var reversed = false { didSet { rebuild() } }
    var query = "" { didSet { rebuild() } }
    var showStats = true
    var interval: Double = 2.0

    // system stats
    var memory: MemoryInfo?
    var coreUsage: [Double] = []
    var perfLevels: [CpuKind] = []
    var load: (Double, Double, Double) = (0, 0, 0)
    var battery: BatteryInfo?
    var processCount = 0
    var threadCount = 0

    /// Rolling average-CPU history for the chart.
    private(set) var cpuHistory: [Double] = []
    static let historyCap = 90

    var lastError: String?
    var signalTarget: ProcessSnapshot?   // drives the send-signal confirmation
    var info: InfoState?                  // drives the info panel sheet

    /// State for the info-panel sheet: the target plus the (async-loaded) detail.
    struct InfoState: Identifiable {
        let pid: Int32
        let name: String
        var detail: ProcessDetail?
        var error: String?
        var id: Int32 { pid }
    }

    private var all: [ProcessSnapshot] = []
    private let cpu = CpuTracker()
    private var prevTicks: [CpuTicks] = []
    private var timer: Task<Void, Never>?

    init(source: any ProcessSource) {
        self.source = source
        self.perfLevels = (try? source.cpuPerfLevels()) ?? []
    }

    var selectedProcess: ProcessSnapshot? {
        guard let selection else { return nil }
        return all.first { $0.pid == selection }
    }

    // MARK: Lifecycle

    func start() {
        guard timer == nil else { return }
        timer = Task { [weak self] in
            while !Task.isCancelled {
                await self?.refresh()
                let secs = await self?.interval ?? 2.0
                try? await Task.sleep(for: .seconds(secs))
            }
        }
    }

    func stop() { timer?.cancel(); timer = nil }

    // MARK: Refresh

    func refresh() async {
        let src = source
        do {
            let snaps = try await Task.detached(priority: .userInitiated) {
                try src.snapshotAll()
            }.value
            let ticks = await Task.detached { try? src.cpuTicks() }.value
            let mem = await Task.detached { try? src.memoryInfo() }.value
            let ld = await Task.detached { try? src.systemLoadAverage() }.value
            let bat = await Task.detached { try? src.systemBattery() }.value

            cpu.update(snaps, now: Date())
            all = snaps
            memory = mem
            load = ld ?? (0, 0, 0)
            battery = bat
            processCount = snaps.count
            threadCount = snaps.reduce(0) { $0 + Int($1.activity.threadCount) }

            if let ticks {
                if prevTicks.count == ticks.count && !prevTicks.isEmpty {
                    coreUsage = zip(ticks, prevTicks).map { CoreUsage.percent(current: $0, previous: $1) }
                    let avg = coreUsage.isEmpty ? 0 : coreUsage.reduce(0, +) / Double(coreUsage.count)
                    cpuHistory.append(avg)
                    if cpuHistory.count > Self.historyCap { cpuHistory.removeFirst(cpuHistory.count - Self.historyCap) }
                }
                prevTicks = ticks
            }
            lastError = nil
            rebuild()
        } catch {
            lastError = "\(error)"
        }
    }

    private func rebuild() {
        var list = all
        if !query.isEmpty { list = list.filter { matchesQuery($0, query) } }
        let sorted = sortProcesses(list, by: sort, reversed: reversed, cpu: cpu.percent)
        rows = sorted.map { Row(snapshot: $0, cpu: cpu.percent($0.pid)) }
    }

    // MARK: Actions

    func sendSignal(_ sig: Int32, to pid: Int32) {
        do {
            try source.killProcess(pid, sig)
            lastError = nil
        } catch {
            lastError = "signal failed: \(error)"
        }
        signalTarget = nil
    }

    /// Open the info panel for `pid` and load its full detail off the main actor
    /// (env parsing + network can be slow). The sheet shows a spinner until it lands.
    func openInfo(_ pid: Int32) {
        guard let p = all.first(where: { $0.pid == pid }) else { return }
        let parentName = all.first { $0.pid == p.ppid }?.name ?? ""
        info = InfoState(pid: pid, name: p.name, detail: nil, error: nil)
        let src = source
        Task {
            do {
                let d = try await Task.detached(priority: .userInitiated) {
                    try src.processDetail(pid, parentName: parentName)
                }.value
                if info?.pid == pid { info?.detail = d }
            } catch {
                if info?.pid == pid { info?.error = "\(error)" }
            }
        }
    }

    func openInfoForSelection() {
        if let pid = selection { openInfo(pid) }
    }
}
