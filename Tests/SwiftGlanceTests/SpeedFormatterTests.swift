import XCTest
@testable import SwiftGlance

/// 候选 1 解锁的 test surface：CLAUDE.md §4 阶梯规格 1:1 落成测试。
final class SpeedFormatterTests: XCTestCase {

    private let KB: Double = 1024
    private let MB: Double = 1024 * 1024
    private let GB: Double = 1024 * 1024 * 1024

    func testBytesRange() {
        XCTAssertEqual(SpeedFormatter.format(7).0, "7")
        XCTAssertEqual(SpeedFormatter.format(7).1, "B")
        XCTAssertEqual(SpeedFormatter.format(63).0, "63")
        XCTAssertEqual(SpeedFormatter.format(0).0, "0")
    }

    func testBytesToKilobyteDecimal() {
        // 100–1023 B/s → .XK，且永不出现 .0
        XCTAssertEqual(tuple(SpeedFormatter.format(100)), ".1K")
        XCTAssertEqual(tuple(SpeedFormatter.format(512)), ".5K")
    }

    func testKilobyteRange() {
        XCTAssertEqual(tuple(SpeedFormatter.format(KB)), "1K")
        XCTAssertEqual(tuple(SpeedFormatter.format(45 * KB)), "45K")
        XCTAssertEqual(tuple(SpeedFormatter.format(99 * KB)), "99K")
    }

    func testKilobyteToMegabyteDecimal() {
        XCTAssertEqual(tuple(SpeedFormatter.format(100 * KB)), ".1M")
        XCTAssertEqual(tuple(SpeedFormatter.format(500 * KB)), ".4M")
    }

    func testMegabyteRange() {
        XCTAssertEqual(tuple(SpeedFormatter.format(MB)), "1M")
        XCTAssertEqual(tuple(SpeedFormatter.format(55 * MB)), "55M")
        XCTAssertEqual(tuple(SpeedFormatter.format(99 * MB)), "99M")
    }

    func testMegabyteToGigabyteDecimal() {
        XCTAssertEqual(tuple(SpeedFormatter.format(500 * MB)), ".4G")
    }

    func testGigabyteRange() {
        XCTAssertEqual(tuple(SpeedFormatter.format(GB)), "1G")
        XCTAssertEqual(tuple(SpeedFormatter.format(10 * GB)), "10G")
    }

    /// 跨档小数始终落在 .1–.9，绝不出现 .0。
    func testDecimalNeverZero() {
        for bps in stride(from: 100.0, to: 1024.0, by: 1.0) {
            let (num, _) = SpeedFormatter.format(bps)
            XCTAssertNotEqual(num, ".0", "bps=\(bps) 产生了 .0")
            XCTAssertTrue(num.hasPrefix("."), "bps=\(bps) 应为小数档")
        }
    }

    private func tuple(_ parts: (num: String, unit: String)) -> String {
        parts.num + parts.unit
    }
}
