import Foundation

/// 一次采样的全部指标。
struct Metrics: Equatable {
    var cpuUsage: Double      // 0–100 (%)
    var memoryUsage: Double   // 0–100 (%)
    var memoryUsed: UInt64    // bytes
    var memoryTotal: UInt64   // bytes
    var downloadBps: Double   // bytes/s
    var uploadBps: Double     // bytes/s
}

/// 系统数据采集的对外门面（§3）。
///
/// 深 module，小 interface（`sample() -> Metrics`），实现内部沿一道 seam 拆成：
/// `CounterSource`（读原始计数器，碰 syscall）+ `RateDiffer`（纯差分）。
/// 默认注入活体 adapter；测试可注入脚本化 adapter，无需改动调用方。
final class SystemMonitor {

    private let source: CounterSource
    private let differ = RateDiffer()

    init(source: CounterSource = LiveCounters()) {
        self.source = source
    }

    func sample() -> Metrics {
        differ.diff(source.read())
    }
}
