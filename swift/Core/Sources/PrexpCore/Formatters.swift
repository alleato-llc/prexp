import Foundation

/// Output formats for non-UI mode — mirrors Rust `OutputFormat`.
public enum OutputFormat: String, Sendable, CaseIterable {
    case json
    case tsv
}

public enum Formatters {
    /// Grouped JSON — a pretty-printed array of `ProcessSnapshot`, matching Rust's
    /// `serde_json::to_writer_pretty`. Field order follows the model declaration
    /// order (not sorted), and slashes are not escaped, to match serde's bytes.
    public static func json(_ snapshots: [ProcessSnapshot]) throws -> String {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .withoutEscapingSlashes]
        let data = try encoder.encode(snapshots)
        return String(decoding: data, as: UTF8.self)
    }

    /// TSV: one row per open resource — mirrors Rust's `tsv::format`.
    public static func tsv(_ snapshots: [ProcessSnapshot]) -> String {
        var out = "PID\tPROCESS\tDESCRIPTOR\tKIND\tPATH\n"
        for proc in snapshots {
            for res in proc.resources {
                let path = res.path ?? "-"
                out += "\(proc.pid)\t\(proc.name)\t\(res.descriptor)\t\(res.kind.rawValue)\t\(path)\n"
            }
        }
        return out
    }
}
