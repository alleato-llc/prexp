import XCTest
@testable import PrexpCore

// Pure-logic tests over canned data (no live system) — these are the kind of
// checks the shared `spec/` parity oracle will eventually assert against both
// implementations.
final class FormatterTests: XCTestCase {

    private func sample() -> [ProcessSnapshot] {
        [
            ProcessSnapshot(
                pid: 100, ppid: 1, name: "alpha", state: .running, accessible: true,
                memory: ProcessMemory(rss: 2048, phys: 1024),
                activity: ProcessActivity(cpuTimeNs: 5_000_000, threadCount: 4, faults: 10,
                                          contextSwitches: 20, syscallsMach: 30, syscallsUnix: 40),
                diskIo: DiskIo(bytesRead: 512, bytesWritten: 256),
                resources: [
                    OpenResource(descriptor: 0, kind: .device, path: "/dev/null"),
                    OpenResource(descriptor: 3, kind: .file, path: "/tmp/a.txt"),
                    OpenResource(descriptor: 4, kind: .socket, path: nil),
                ]),
            ProcessSnapshot(
                pid: 101, ppid: 100, name: "beta", state: .sleeping, accessible: false,
                memory: ProcessMemory(rss: 0, phys: 0),
                activity: ProcessActivity(cpuTimeNs: 0, threadCount: 1, faults: 0,
                                          contextSwitches: 0, syscallsMach: 0, syscallsUnix: 0),
                diskIo: DiskIo(bytesRead: 0, bytesWritten: 0),
                resources: []),
        ]
    }

    func testTsvMatchesRustLayout() {
        let tsv = Formatters.tsv(sample())
        let lines = tsv.split(separator: "\n", omittingEmptySubsequences: false)
        XCTAssertEqual(lines.first, "PID\tPROCESS\tDESCRIPTOR\tKIND\tPATH")
        XCTAssertEqual(lines[1], "100\talpha\t0\tdevice\t/dev/null")
        XCTAssertEqual(lines[2], "100\talpha\t3\tfile\t/tmp/a.txt")
        XCTAssertEqual(lines[3], "100\talpha\t4\tsocket\t-")  // nil path → "-"
        // beta has no resources → no rows.
        XCTAssertEqual(lines.filter { $0.hasPrefix("101") }.count, 0)
    }

    func testJsonHasSnakeCaseKeysAndNullPath() throws {
        let json = try Formatters.json(sample())
        XCTAssertTrue(json.contains("\"cpu_time_ns\""))
        XCTAssertTrue(json.contains("\"context_switches\""))
        XCTAssertTrue(json.contains("\"disk_io\""))
        XCTAssertTrue(json.contains("\"path\" : null"))  // socket resource
        XCTAssertTrue(json.contains("\"state\" : \"running\""))
        XCTAssertTrue(json.contains("\"kind\" : \"device\""))
    }

    func testJsonRoundTrips() throws {
        let json = try Formatters.json(sample())
        let decoded = try JSONDecoder().decode([ProcessSnapshot].self, from: Data(json.utf8))
        XCTAssertEqual(decoded, sample())
    }

    func testCountByKind() {
        let p = sample()[0]
        XCTAssertEqual(p.count(of: .file), 1)
        XCTAssertEqual(p.count(of: .socket), 1)
        XCTAssertEqual(p.count(of: .pipe), 0)
    }

    func testStateLabels() {
        XCTAssertEqual(ProcessState.running.label, "RUN")
        XCTAssertEqual(ProcessState.zombie.label, "ZMB")
        XCTAssertEqual(ProcessState.unknown.label, "???")
    }
}
