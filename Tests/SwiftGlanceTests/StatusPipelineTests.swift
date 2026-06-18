import XCTest
@testable import SwiftGlance

/// 候选 3 解锁的 test surface：注入脚本化 sampler，
/// 脱离 AppKit 生命周期验证「Metrics → NSImage」管线。
final class StatusPipelineTests: XCTestCase {

    func testTickRendersNonEmptyImage() {
        let stub = Metrics(
            cpuUsage: 42, memoryUsage: 57,
            memoryUsed: 0, memoryTotal: 0,
            downloadBps: 27 * 1024, uploadBps: 400 * 1024
        )
        let pipeline = StatusPipeline(sample: { stub })

        let image = pipeline.tick()

        XCTAssertGreaterThan(image.size.width, 0)
        XCTAssertGreaterThan(image.size.height, 0)
        XCTAssertTrue(image.isTemplate, "状态栏图标应为 template，交系统处理深/浅色反相")
    }

    func testTickSamplesEachCall() {
        var calls = 0
        let pipeline = StatusPipeline(sample: {
            calls += 1
            return Metrics(cpuUsage: 0, memoryUsage: 0, memoryUsed: 0,
                           memoryTotal: 0, downloadBps: 0, uploadBps: 0)
        })

        _ = pipeline.tick()
        _ = pipeline.tick()

        XCTAssertEqual(calls, 2, "每次 tick 应采样一次")
    }
}
