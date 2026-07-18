import Foundation

/// Human-readable formatting helpers, matching the compact style of the Rust TUI
/// (e.g. `11.3M`, `1.5G`, `28.4`).
public enum Format {
    /// Bytes → compact binary units (base 1024): `B`, `K`, `M`, `G`, `T`.
    public static func bytes(_ n: UInt64) -> String {
        let units = ["B", "K", "M", "G", "T", "P"]
        var value = Double(n)
        var unit = 0
        while value >= 1024 && unit < units.count - 1 {
            value /= 1024
            unit += 1
        }
        if unit == 0 { return "\(n)B" }
        return String(format: "%.1f%@", value, units[unit])
    }

    /// A CPU/percent value with one decimal (e.g. `28.4`).
    public static func percent(_ p: Double) -> String {
        String(format: "%.1f", p)
    }

    /// Nanoseconds → `Hh Mm`, `Mm Ss`, or `Ss` (cumulative CPU time).
    public static func durationNs(_ ns: UInt64) -> String {
        let secs = ns / 1_000_000_000
        let h = secs / 3600, m = (secs % 3600) / 60, s = secs % 60
        if h > 0 { return "\(h)h\(m)m" }
        if m > 0 { return "\(m)m\(s)s" }
        return "\(s)s"
    }
}
