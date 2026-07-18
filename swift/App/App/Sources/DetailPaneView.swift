import PrexpCore
import SwiftUI

/// The selected process's open resources (FD / KIND / PATH), read straight off
/// the loaded snapshot — the GUI analogue of the TUI detail overlay.
struct DetailPaneView: View {
    let process: ProcessSnapshot?

    var body: some View {
        if let p = process {
            VStack(alignment: .leading, spacing: 0) {
                header(p)
                Divider()
                if p.resources.isEmpty {
                    empty(p.accessible ? "No open resources" : "No access to this process")
                } else {
                    Table(p.resources.enumerated().map { IndexedResource(index: $0.offset, resource: $0.element) }) {
                        TableColumn("FD") { Text("\($0.resource.descriptor)").monospacedDigit() }.width(44)
                        TableColumn("Kind") { Text($0.resource.kind.rawValue).foregroundStyle(color(for: $0.resource.kind)) }.width(72)
                        TableColumn("Path") { Text($0.resource.path ?? "—").textSelection(.enabled).lineLimit(1) }
                    }
                    .tableStyle(.inset)
                }
            }
        } else {
            empty("Select a process")
        }
    }

    private func header(_ p: ProcessSnapshot) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(p.name).font(.headline).lineLimit(1)
                Text("pid \(p.pid)").font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
                Spacer()
                Text(p.state.label).font(.caption.bold())
                    .padding(.horizontal, 6).padding(.vertical, 2)
                    .background(.quaternary, in: Capsule())
            }
            HStack(spacing: 12) {
                stat("RSS", Format.bytes(p.memory.rss))
                stat("Private", Format.bytes(p.memory.phys))
                stat("Threads", "\(p.activity.threadCount)")
                stat("CPU time", Format.durationNs(p.activity.cpuTimeNs))
            }
        }
        .padding(10)
    }

    private func stat(_ name: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(name).font(.caption2).foregroundStyle(.secondary)
            Text(value).font(.caption.monospacedDigit())
        }
    }

    private func empty(_ text: String) -> some View {
        VStack { Spacer(); Text(text).foregroundStyle(.secondary); Spacer() }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func color(for kind: ResourceKind) -> Color {
        switch kind {
        case .file: return .primary
        case .socket: return .blue
        case .pipe: return .purple
        case .device: return .orange
        case .kqueue: return .teal
        case .unknown: return .secondary
        }
    }
}

private struct IndexedResource: Identifiable {
    let index: Int
    let resource: OpenResource
    var id: Int { index }
}
