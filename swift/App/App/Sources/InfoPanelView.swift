import AppKit
import PrexpCore
import SwiftUI

/// The tabbed info panel (Overview / Resources / Network / Environment) over a
/// process's full `ProcessDetail` — the GUI analogue of the Rust prexp-desktop
/// info panel. Presented as a sheet; loads asynchronously with a spinner.
struct InfoPanelView: View {
    @Bindable var state: AppState
    let info: AppState.InfoState

    @State private var tab: Tab = .overview
    @State private var envFilter = ""

    enum Tab: String, CaseIterable, Identifiable {
        case overview = "Overview", resources = "Resources", network = "Network", environment = "Environment"
        var id: String { rawValue }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            Group {
                if let d = info.detail {
                    content(d)
                } else if let e = info.error {
                    message("Couldn't load details:\n\(e)", systemImage: "exclamationmark.triangle")
                } else {
                    ProgressView("Loading…").frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        }
        .frame(width: 640, height: 480)
    }

    // MARK: Header

    private var header: some View {
        VStack(spacing: 8) {
            HStack {
                Text(info.name).font(.headline).lineLimit(1)
                Text("pid \(info.pid)").font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
                Spacer()
                Button("Done") { state.info = nil }.keyboardShortcut(.defaultAction)
            }
            Picker("", selection: $tab) {
                ForEach(Tab.allCases) { Text($0.rawValue).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
        }
        .padding(12)
    }

    @ViewBuilder
    private func content(_ d: ProcessDetail) -> some View {
        switch tab {
        case .overview: overview(d)
        case .resources: resources(d)
        case .network: network(d)
        case .environment: environment(d)
        }
    }

    // MARK: Overview

    private func overview(_ d: ProcessDetail) -> some View {
        Form {
            row("PID", "\(d.pid)")
            row("Parent", "\(d.parentName) (\(d.ppid))")
            row("User", "\(d.user) (\(d.uid))")
            row("State", d.state.label)
            row("Nice", "\(d.nice)")
            row("Started", started(d.startedSecs))
            row("Path", d.path.isEmpty ? "—" : d.path, selectable: true)
            row("CWD", d.cwd.isEmpty ? "—" : d.cwd, selectable: true)
        }
        .formStyle(.grouped)
    }

    // MARK: Resources

    private func resources(_ d: ProcessDetail) -> some View {
        Form {
            Section("Memory") {
                row("RSS", Format.bytes(d.memoryRss))
                row("Private", Format.bytes(d.memoryPhys))
                row("Virtual", Format.bytes(d.virtualSize))
            }
            Section("CPU / scheduling") {
                row("CPU time", Format.durationNs(d.cpuTimeNs))
                row("Threads", "\(d.threadCount)")
                row("Faults", "\(d.faults)")
                row("Context switches", "\(d.contextSwitches)")
                row("Syscalls (mach / unix)", "\(d.syscallsMach) / \(d.syscallsUnix)")
            }
            Section("Disk I/O") {
                row("Read", Format.bytes(d.diskBytesRead))
                row("Written", Format.bytes(d.diskBytesWritten))
            }
            Section("File descriptors") {
                row("Files", "\(d.fdFiles)")
                row("Sockets", "\(d.fdSockets)")
                row("Pipes", "\(d.fdPipes)")
                row("Other", "\(d.fdOther)")
                row("Total", "\(d.fdTotal)")
            }
        }
        .formStyle(.grouped)
    }

    // MARK: Network

    private func network(_ d: ProcessDetail) -> some View {
        Group {
            if d.network.isEmpty {
                message("No network connections", systemImage: "network.slash")
            } else {
                Table(d.network.enumerated().map { IdxConn(id: $0.offset, c: $0.element) }) {
                    TableColumn("Proto") { Text($0.c.proto) }.width(56)
                    TableColumn("Local") { Text($0.c.localAddr).monospacedDigit().textSelection(.enabled) }
                    TableColumn("Remote") { Text($0.c.remoteAddr ?? "—").monospacedDigit().textSelection(.enabled) }
                    TableColumn("State") { Text($0.c.state ?? "—").foregroundStyle(.secondary) }.width(120)
                }
            }
        }
    }

    // MARK: Environment

    private func environment(_ d: ProcessDetail) -> some View {
        let items = d.environment.enumerated()
            .map { IdxEnv(id: $0.offset, e: $0.element) }
            .filter { envFilter.isEmpty
                || $0.e.key.localizedCaseInsensitiveContains(envFilter)
                || $0.e.value.localizedCaseInsensitiveContains(envFilter) }
        return VStack(spacing: 0) {
            HStack {
                TextField("Filter", text: $envFilter).textFieldStyle(.roundedBorder)
                Button {
                    copy(d.environment.map { "\($0.key)=\($0.value)" }.joined(separator: "\n"))
                } label: { Label("Copy all", systemImage: "doc.on.doc") }
            }
            .padding(8)
            Divider()
            if items.isEmpty {
                message(d.environment.isEmpty ? "No environment (or no access)" : "No matches",
                        systemImage: "leaf")
            } else {
                List(items) { item in
                    HStack(alignment: .firstTextBaseline) {
                        Text(item.e.key).font(.caption.monospaced().bold())
                        Text(item.e.value).font(.caption.monospaced()).foregroundStyle(.secondary)
                            .textSelection(.enabled).lineLimit(1)
                    }
                    .contextMenu {
                        Button("Copy KEY=VALUE") { copy("\(item.e.key)=\(item.e.value)") }
                        Button("Copy value") { copy(item.e.value) }
                    }
                }
            }
        }
    }

    // MARK: Helpers

    private func row(_ name: String, _ value: String, selectable: Bool = false) -> some View {
        LabeledContent(name) {
            Text(value).monospacedDigit()
                .textSelection(.enabled)
                .lineLimit(1).truncationMode(.middle)
        }
    }

    private func started(_ secs: UInt64) -> String {
        guard secs > 0 else { return "—" }
        return Date(timeIntervalSince1970: Double(secs)).formatted(date: .abbreviated, time: .shortened)
    }

    private func message(_ text: String, systemImage: String) -> some View {
        ContentUnavailableView(text, systemImage: systemImage)
    }

    private func copy(_ s: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(s, forType: .string)
    }
}

private struct IdxConn: Identifiable { let id: Int; let c: NetworkConnection }
private struct IdxEnv: Identifiable { let id: Int; let e: EnvVar }
