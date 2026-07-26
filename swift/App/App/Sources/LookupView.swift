import PrexpCore
import SwiftUI

/// Reverse path lookup — enter a path, get the processes that have it open
/// (`findByPath`). The GUI analogue of the Rust prexp-desktop lookup panel.
/// Double-clicking a result selects it in the main table and closes the sheet.
struct LookupView: View {
    @Bindable var state: AppState

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            searchBar
            Divider()
            results
        }
        .frame(width: 620, height: 440)
    }

    private var header: some View {
        HStack {
            Label("Find processes by open path", systemImage: "magnifyingglass")
                .font(.headline)
            Spacer()
            Button("Done") { state.showLookup = false }.keyboardShortcut(.cancelAction)
        }
        .padding(12)
    }

    private var searchBar: some View {
        HStack {
            TextField("/path/to/file, socket, or device", text: $state.lookupQuery)
                .textFieldStyle(.roundedBorder)
                .onSubmit { state.runLookup() }
            Button("Search") { state.runLookup() }
                .keyboardShortcut(.defaultAction)
                .disabled(state.lookupQuery.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .padding(12)
    }

    @ViewBuilder
    private var results: some View {
        if state.lookupSearching {
            ProgressView("Searching…").frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if !state.lookupSearched {
            ContentUnavailableView("Enter a path to find the processes holding it open",
                                   systemImage: "doc.text.magnifyingglass")
        } else if state.lookupResults.isEmpty {
            ContentUnavailableView("No processes have that path open", systemImage: "xmark.circle")
        } else {
            VStack(alignment: .leading, spacing: 0) {
                Text("\(state.lookupResults.count) process\(state.lookupResults.count == 1 ? "" : "es")")
                    .font(.caption).foregroundStyle(.secondary).padding(.horizontal, 12).padding(.top, 6)
                Table(state.lookupResults.map(Row.init)) {
                    TableColumn("PID") { Text("\($0.snapshot.pid)").monospacedDigit() }.width(64)
                    TableColumn("Name") { Text($0.snapshot.name).lineLimit(1) }
                    TableColumn("Files") { Text("\($0.snapshot.count(of: .file))").monospacedDigit() }.width(56)
                    TableColumn("FDs") { Text("\($0.snapshot.resources.count)").monospacedDigit() }.width(56)
                }
                .contextMenu(forSelectionType: Int32.self) { pids in
                    if let pid = pids.first { Button("Reveal in list") { reveal(pid) } }
                } primaryAction: { pids in
                    if let pid = pids.first { reveal(pid) }
                }
            }
        }
    }

    private func reveal(_ pid: Int32) {
        state.selection = pid
        state.showLookup = false
    }

    private struct Row: Identifiable {
        let snapshot: ProcessSnapshot
        var id: Int32 { snapshot.pid }
    }
}
