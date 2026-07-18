import PrexpCore
import SwiftUI

struct ContentView: View {
    @Bindable var state: AppState

    var body: some View {
        VStack(spacing: 0) {
            if state.showStats {
                StatsHeaderView(state: state)
                Divider()
            }
            HSplitView {
                processTable
                    .frame(minWidth: 520)
                DetailPaneView(process: state.selectedProcess)
                    .frame(minWidth: 260)
            }
            Divider()
            statusBar
        }
        .toolbar { toolbar }
        .confirmationDialog(
            state.signalTarget.map { "Signal \($0.name) (pid \($0.pid))?" } ?? "",
            isPresented: Binding(get: { state.signalTarget != nil },
                                 set: { if !$0 { state.signalTarget = nil } }),
            presenting: state.signalTarget
        ) { p in
            Button("Terminate — SIGTERM") { state.sendSignal(SIGTERM, to: p.pid) }
            Button("Interrupt — SIGINT") { state.sendSignal(SIGINT, to: p.pid) }
            Button("Force Kill — SIGKILL", role: .destructive) { state.sendSignal(SIGKILL, to: p.pid) }
            Button("Cancel", role: .cancel) { state.signalTarget = nil }
        }
    }

    // MARK: Process table

    private var processTable: some View {
        Table(state.rows, selection: $state.selection) {
            TableColumn("PID") { r in Text("\(r.snapshot.pid)").monospacedDigit() }.width(58)
            TableColumn("Name") { r in
                Text(r.snapshot.name)
                    .lineLimit(1)
                    .foregroundStyle(r.snapshot.accessible ? .primary : .secondary)
            }
            TableColumn("CPU%") { r in
                Text(Format.percent(r.cpu)).monospacedDigit().foregroundStyle(loadColor(r.cpu))
            }.width(56)
            TableColumn("Memory") { r in Text(Format.bytes(r.snapshot.memory.rss)).monospacedDigit() }.width(74)
            TableColumn("Private") { r in Text(Format.bytes(r.snapshot.memory.phys)).monospacedDigit() }.width(74)
            TableColumn("Thr") { r in Text("\(r.snapshot.activity.threadCount)").monospacedDigit() }.width(44)
            TableColumn("Files") { r in Text("\(r.snapshot.count(of: .file))").monospacedDigit() }.width(48)
            TableColumn("FDs") { r in Text("\(r.snapshot.resources.count)").monospacedDigit() }.width(48)
        }
        .contextMenu(forSelectionType: Int32.self) { pids in
            if let pid = pids.first, let p = state.rows.first(where: { $0.id == pid })?.snapshot {
                Button("Send Signal…") { state.signalTarget = p }
            }
        }
        .tableStyle(.inset)
    }

    // MARK: Toolbar

    @ToolbarContentBuilder
    private var toolbar: some ToolbarContent {
        ToolbarItemGroup {
            TextField("Filter", text: $state.query)
                .textFieldStyle(.roundedBorder)
                .frame(width: 170)

            Picker("Sort", selection: $state.sort) {
                ForEach(SortField.allCases, id: \.self) { Text($0.label).tag($0) }
            }
            .frame(width: 110)

            Button { state.reversed.toggle() } label: {
                Image(systemName: state.reversed ? "arrow.up" : "arrow.down")
            }
            .help("Reverse sort direction")

            Toggle(isOn: $state.showStats) { Image(systemName: "chart.bar.xaxis") }
                .help("Show system stats")

            Button { Task { await state.refresh() } } label: { Image(systemName: "arrow.clockwise") }
                .help("Refresh now")
        }
    }

    // MARK: Status bar

    private var statusBar: some View {
        HStack(spacing: 12) {
            if let err = state.lastError {
                Label(err, systemImage: "exclamationmark.triangle").foregroundStyle(.red).lineLimit(1)
            }
            Spacer()
            Text("\(state.processCount) processes")
            Text("\(state.threadCount) threads")
        }
        .font(.caption.monospacedDigit())
        .foregroundStyle(.secondary)
        .padding(.horizontal, 10).padding(.vertical, 4)
    }
}
