import Charts
import PrexpCore
import SwiftUI

/// System stats header — memory, per-core usage, load/battery, and a CPU-history
/// line chart (Swift Charts). Mirrors the Rust `prexp-desktop` stats header.
struct StatsHeaderView: View {
    let state: AppState

    var body: some View {
        HStack(alignment: .top, spacing: 16) {
            memory.frame(width: 210)
            cores.frame(maxWidth: .infinity)
            VStack(alignment: .leading, spacing: 8) {
                info
                chart.frame(height: 46)
            }
            .frame(width: 240)
        }
        .padding(12)
        .background(.regularMaterial)
    }

    // MARK: Memory

    private var memory: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("MEMORY").font(.caption2.bold()).foregroundStyle(.secondary)
            if let m = state.memory, m.total > 0 {
                let frac = Double(m.used) / Double(m.total)
                ProgressView(value: min(frac, 1)) {
                    HStack {
                        Text("\(Format.bytes(m.used)) / \(Format.bytes(m.total))")
                        Spacer()
                        Text("\(Int(frac * 100))%")
                    }
                    .font(.caption.monospacedDigit())
                }
                .tint(loadColor(frac * 100))
                HStack(spacing: 10) {
                    label("wired", m.wired)
                    label("compressed", m.compressed)
                }
                if m.swapUsed > 0 { label("swap", m.swapUsed) }
            } else {
                Text("—").foregroundStyle(.secondary)
            }
        }
    }

    private func label(_ name: String, _ bytes: UInt64) -> some View {
        Text("\(name) \(Format.bytes(bytes))")
            .font(.caption2.monospacedDigit()).foregroundStyle(.secondary)
    }

    // MARK: Per-core grid

    private var cores: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("CPU CORES").font(.caption2.bold()).foregroundStyle(.secondary)
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 6), count: 8), spacing: 4) {
                ForEach(Array(state.coreUsage.enumerated()), id: \.offset) { idx, usage in
                    VStack(spacing: 1) {
                        RoundedRectangle(cornerRadius: 2)
                            .fill(loadColor(usage).opacity(0.25))
                            .overlay(alignment: .bottom) {
                                GeometryReader { geo in
                                    RoundedRectangle(cornerRadius: 2)
                                        .fill(loadColor(usage))
                                        .frame(height: geo.size.height * usage / 100)
                                        .frame(maxHeight: .infinity, alignment: .bottom)
                                }
                            }
                            .frame(height: 18)
                        Text("\(coreTag(idx))\(idx)").font(.system(size: 8)).monospacedDigit()
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
    }

    private func coreTag(_ i: Int) -> String {
        guard i < state.perfLevels.count else { return "" }
        switch state.perfLevels[i] {
        case .performance: return "P"
        case .efficiency: return "E"
        case .unknown: return ""
        }
    }

    // MARK: Load / battery / counts

    private var info: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(String(format: "load  %.2f  %.2f  %.2f", state.load.0, state.load.1, state.load.2))
                .font(.caption.monospacedDigit())
            if let b = state.battery {
                Text(String(format: "battery  %.0f%%%@", b.percent, b.charging ? " ⚡" : ""))
                    .font(.caption.monospacedDigit())
            }
            Text("\(state.processCount) procs · \(state.threadCount) threads")
                .font(.caption2.monospacedDigit()).foregroundStyle(.secondary)
        }
    }

    // MARK: CPU history chart

    private var chart: some View {
        Chart(Array(state.cpuHistory.enumerated()), id: \.offset) { i, v in
            AreaMark(x: .value("t", i), y: .value("cpu", v))
                .foregroundStyle(.linearGradient(colors: [Color.accentColor.opacity(0.4), .clear],
                                                 startPoint: .top, endPoint: .bottom))
            LineMark(x: .value("t", i), y: .value("cpu", v))
                .foregroundStyle(Color.accentColor)
                .interpolationMethod(.catmullRom)
        }
        .chartYScale(domain: 0...100)
        .chartXAxis(.hidden)
        .chartYAxis {
            AxisMarks(values: [0, 50, 100]) { AxisGridLine(); AxisValueLabel() }
        }
    }
}
