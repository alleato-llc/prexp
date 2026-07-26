import Tint

/// A prexp color theme conforming to tint's `Theme`, plus load-grading colors for
/// CPU% cells. A small set to start (the full picker can come later).
public struct PrexpTheme: Theme, Sendable {
    public let name: String
    public var primary: Style
    public var secondary: Style
    public var highlight: Style
    public var accent: Style
    public var muted: Style
    public var border: Style
    public var title: Style
    public var error: Style
    public var statusBar: Style

    /// Colors used to grade a CPU% (low → high).
    public var loadLow: Color
    public var loadMid: Color
    public var loadHigh: Color

    /// Grade a percentage (0…100) into a color.
    public func loadColor(_ pct: Double) -> Color {
        if pct >= 70 { return loadHigh }
        if pct >= 30 { return loadMid }
        return loadLow
    }

    public static let dark = PrexpTheme(
        name: "Default",
        primary: Style(fg: .white),
        secondary: Style(fg: .brightBlack),
        highlight: Style(fg: .black, bg: .cyan, bold: true),
        accent: Style(fg: .cyan, bold: true),
        muted: Style(fg: .brightBlack, dim: true),
        border: Style(fg: .brightBlack),
        title: Style(fg: .cyan, bold: true),
        error: Style(fg: .red, bold: true),
        statusBar: Style(fg: .black, bg: .cyan),
        loadLow: .green, loadMid: .yellow, loadHigh: .red)
}
