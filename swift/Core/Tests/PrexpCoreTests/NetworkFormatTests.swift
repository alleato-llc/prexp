import XCTest
@testable import PrexpCore

// Pure formatting logic for the network table — mirrors Rust `format_sock_addr`
// / `tcp_state_name`. (Live socket parsing is exercised against the real system
// via prexp-smoke; here we pin the deterministic string formatting.)
final class NetworkFormatTests: XCTestCase {

    /// Build a little-endian UInt32 whose memory bytes are [a,b,c,d] — the same
    /// order `format_sock_addr` reads the 4 address bytes.
    private func sAddr(_ a: UInt8, _ b: UInt8, _ c: UInt8, _ d: UInt8) -> UInt32 {
        UInt32(a) | UInt32(b) << 8 | UInt32(c) << 16 | UInt32(d) << 24
    }

    func testFormatIPv4WithPort() {
        XCTAssertEqual(NativeSource.formatSockAddr(sAddr(192, 168, 1, 71), port: 443, vflag: 0x1),
                       "192.168.1.71:443")
    }

    func testZeroAddressIsStar() {
        XCTAssertEqual(NativeSource.formatSockAddr(0, port: 0, vflag: 0x1), "*:*")
    }

    func testZeroPortIsStar() {
        XCTAssertEqual(NativeSource.formatSockAddr(sAddr(10, 0, 0, 1), port: 0, vflag: 0x1), "10.0.0.1:*")
    }

    func testIPv6Simplified() {
        // vflag without the IPv4 bit → address renders as "*"
        XCTAssertEqual(NativeSource.formatSockAddr(sAddr(1, 2, 3, 4), port: 8080, vflag: 0x2), "*:8080")
    }

    func testTcpStateNames() {
        XCTAssertEqual(NativeSource.tcpStateName(1), "LISTEN")
        XCTAssertEqual(NativeSource.tcpStateName(4), "ESTABLISHED")
        XCTAssertEqual(NativeSource.tcpStateName(10), "TIME_WAIT")
        XCTAssertEqual(NativeSource.tcpStateName(99), "UNKNOWN")
    }
}
