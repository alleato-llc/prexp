import Foundation
import PrexpCore

// Smoke tool: dumps the native process source as JSON or TSV, to eyeball parity
// against `cargo run -p prexp -- --output json|tsv`. Not the real CLI — the Swift
// TUI (`prexp`) lands in the next milestone.
//
// Usage:
//   prexp-smoke [json|tsv]        dump all processes (default: json)
//   prexp-smoke pid <PID>         one process snapshot as JSON
//   prexp-smoke detail <PID>      process detail as JSON
//   prexp-smoke system            system metrics summary

let source = NativeSource()
let args = Array(CommandLine.arguments.dropFirst())

do {
    switch args.first {
    case "tsv":
        print(Formatters.tsv(try source.snapshotAll()), terminator: "")

    case "pid":
        guard let pid = args.dropFirst().first.flatMap({ Int32($0) }) else {
            FileHandle.standardError.write(Data("usage: prexp-smoke pid <PID>\n".utf8)); exit(2)
        }
        print(try Formatters.json([try source.snapshotPid(pid)]))

    case "detail":
        guard let pid = args.dropFirst().first.flatMap({ Int32($0) }) else {
            FileHandle.standardError.write(Data("usage: prexp-smoke detail <PID>\n".utf8)); exit(2)
        }
        let detail = try source.processDetail(pid, parentName: "")
        let enc = JSONEncoder(); enc.outputFormatting = [.prettyPrinted, .withoutEscapingSlashes]
        print(String(decoding: try enc.encode(detail), as: UTF8.self))

    case "lookup":
        guard let path = args.dropFirst().first else {
            FileHandle.standardError.write(Data("usage: prexp-smoke lookup <PATH>\n".utf8)); exit(2)
        }
        print(try Formatters.json(try source.findByPath(path)))

    case "system":
        let mem = try source.memoryInfo()
        let ticks = try source.cpuTicks()
        let levels = (try? source.cpuPerfLevels()) ?? []
        let load = (try? source.systemLoadAverage()) ?? (0, 0, 0)
        print("cores: \(ticks.count)  perf-levels: \(levels)")
        print("memory: total=\(mem.total) used=\(mem.used) free=\(mem.free) wired=\(mem.wired) compressed=\(mem.compressed)")
        print("swap: total=\(mem.swapTotal) used=\(mem.swapUsed)")
        print("load: \(load.0) \(load.1) \(load.2)")
        if let boot = try? source.systemBootTimeSecs() { print("boot: \(boot)") }
        if let bat = try? source.systemBattery() {
            print("battery: \(bat.percent)% charging=\(bat.charging)")
        } else {
            print("battery: none")
        }

    default:  // "json" or nothing
        print(try Formatters.json(try source.snapshotAll()))
    }
} catch {
    FileHandle.standardError.write(Data("error: \(error)\n".utf8))
    exit(1)
}
