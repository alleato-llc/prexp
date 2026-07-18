import PrexpCore
import SwiftUI

@main
struct PrexpApp: App {
    @State private var state = AppState(source: NativeSource())

    var body: some Scene {
        WindowGroup {
            ContentView(state: state)
                .frame(minWidth: 920, minHeight: 580)
                .task { state.start() }
                .onDisappear { state.stop() }
        }
        .windowResizability(.contentMinSize)
    }
}

/// Grade a CPU/usage percentage (0…100) into a color, matching the TUI/desktop.
func loadColor(_ pct: Double) -> Color {
    if pct >= 70 { return .red }
    if pct >= 30 { return .orange }
    return .green
}
