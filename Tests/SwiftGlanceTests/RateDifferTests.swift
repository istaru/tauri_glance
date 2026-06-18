import XCTest
@testable import SwiftGlance

/// 候选 2 解锁的 test surface：通过注入脚本化 adapter，
/// 验证差分逻辑的 edge case——无需碰活体硬件。
/// `SystemMonitor(source:)` 接受第二个 adapter，证明这道 seam 名副其实。
final class RateDifferTests: XCTestCase {

    // 脚本化 adapter：按序吐出预设读数，最后一条之后保持不变。
    private final class ScriptedCounters: CounterSource {
        private let readings: [RawCounters]
        private var idx = 0
        init(_ readings: [RawCounters]) { self.readings = readings }
        func read() -> RawCounters {
            defer { idx = min(idx + 1, readings.count - 1) }
            return readings[idx]
        }
    }

    private func raw(
        cpu: [RawCounters.CPUTicks] = [],
        used: UInt64 = 0,
        total: UInt64 = 0,
        rx: UInt64? = nil,
        tx: UInt64? = nil
    ) -> RawCounters {
        RawCounters(cpu: cpu, memoryUsed: used, memoryTotal: total, rxBytes: rx, txBytes: tx)
    }

    // MARK: - 网络

    func testFirstNetworkSampleIsZero() {
        let monitor = SystemMonitor(source: ScriptedCounters([raw(rx: 1000, tx: 1000)]))
        let m = monitor.sample()
        XCTAssertEqual(m.downloadBps, 0)
        XCTAssertEqual(m.uploadBps, 0)
    }

    func testNetworkDiffIsByteDelta() {
        let monitor = SystemMonitor(source: ScriptedCounters([
            raw(rx: 1000, tx: 2000),
            raw(rx: 1500, tx: 2200),
        ]))
        _ = monitor.sample()                 // 首样本归零
        let m = monitor.sample()
        XCTAssertEqual(m.downloadBps, 500)
        XCTAssertEqual(m.uploadBps, 200)
    }

    func testNetworkWraparoundClampsToZero() {
        let monitor = SystemMonitor(source: ScriptedCounters([
            raw(rx: 1000, tx: 1000),
            raw(rx: 500, tx: 400),           // 计数回绕：差为负
        ]))
        _ = monitor.sample()
        let m = monitor.sample()
        XCTAssertEqual(m.downloadBps, 0)
        XCTAssertEqual(m.uploadBps, 0)
    }

    func testNetworkFailurePreservesHistory() {
        let monitor = SystemMonitor(source: ScriptedCounters([
            raw(rx: 1000, tx: 1000),         // 建立历史
            raw(rx: nil, tx: nil),           // 读取失败：速度 0，历史不动
            raw(rx: 2000, tx: 2000),         // 应相对 1000 而非 0 差分
        ]))
        _ = monitor.sample()
        let failed = monitor.sample()
        XCTAssertEqual(failed.downloadBps, 0)
        let recovered = monitor.sample()
        XCTAssertEqual(recovered.downloadBps, 1000, "失败样本不应污染历史")
    }

    // MARK: - CPU

    func testFirstCPUSampleIsZero() {
        let monitor = SystemMonitor(source: ScriptedCounters([
            raw(cpu: [.init(user: 10, system: 0, nice: 0, idle: 10)]),
        ]))
        XCTAssertEqual(monitor.sample().cpuUsage, 0)
    }

    func testCPUAveragesAcrossCores() {
        let monitor = SystemMonitor(source: ScriptedCounters([
            raw(cpu: [
                .init(user: 0, system: 0, nice: 0, idle: 0),
                .init(user: 0, system: 0, nice: 0, idle: 0),
            ]),
            raw(cpu: [
                .init(user: 50, system: 0, nice: 0, idle: 50),  // 50% busy
                .init(user: 75, system: 0, nice: 0, idle: 25),  // 75% busy
            ]),
        ]))
        _ = monitor.sample()
        XCTAssertEqual(monitor.sample().cpuUsage, 62.5, accuracy: 0.001)
    }

    // MARK: - 内存

    func testMemoryPercent() {
        let monitor = SystemMonitor(source: ScriptedCounters([
            raw(used: 8_000_000_000, total: 16_000_000_000),
        ]))
        let m = monitor.sample()
        XCTAssertEqual(m.memoryUsage, 50, accuracy: 0.001)
        XCTAssertEqual(m.memoryUsed, 8_000_000_000)
        XCTAssertEqual(m.memoryTotal, 16_000_000_000)
    }
}
