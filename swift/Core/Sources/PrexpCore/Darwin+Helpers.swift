import Darwin

/// Cached Mach timebase (numer/denom) for converting Mach absolute-time ticks to
/// nanoseconds — the Apple-Silicon ratio is typically 125:3, Intel 1:1. Mirrors
/// Rust's `mach_ticks_to_ns` (cached once via `OnceLock`).
enum MachTimebase {
    private static let cached: (numer: UInt64, denom: UInt64) = {
        var tb = mach_timebase_info_data_t()
        mach_timebase_info(&tb)
        let n = tb.numer == 0 ? 1 : UInt64(tb.numer)
        let d = tb.denom == 0 ? 1 : UInt64(tb.denom)
        return (n, d)
    }()

    /// `ticks * numer / denom`, computed without 64-bit overflow via a
    /// quotient/remainder split (numer/denom are tiny, so both terms stay small).
    static func toNanos(_ ticks: UInt64) -> UInt64 {
        let (n, d) = cached
        if n == d { return ticks }
        return (ticks / d) &* n &+ ((ticks % d) &* n) / d
    }
}

/// Decode a fixed-size C `char` array (imported into Swift as a homogeneous
/// tuple) up to its first NUL. Uses lossy UTF-8 decoding, matching Rust's
/// `String::from_utf8_lossy`. Bounded by the buffer size — never reads past the
/// tuple even if unterminated.
func cStringTuple<T>(_ tuple: T) -> String {
    var value = tuple
    return withUnsafeBytes(of: &value) { raw in
        let bytes = raw.bindMemory(to: UInt8.self)
        let len = bytes.firstIndex(of: 0) ?? bytes.count
        return String(decoding: bytes[..<len], as: UTF8.self)
    }
}

/// Decode a `[CChar]` buffer up to its first NUL (lossy UTF-8).
func cStringBuffer(_ buffer: [CChar]) -> String {
    let bytes = buffer.prefix { $0 != 0 }.map { UInt8(bitPattern: $0) }
    return String(decoding: bytes, as: UTF8.self)
}
