import AppKit

/// 应用心跳管线：采样 → 渲染成状态栏图标。
///
/// 接收 `sample` 依赖而非自己创建——AppDelegate 只负责用定时器把 `tick()` 跑起来。
/// 注入脚本化 sampler 即可脱离 AppKit 测试「Metrics → NSImage」这条管线。
final class StatusPipeline {

    private let sample: () -> Metrics

    init(sample: @escaping () -> Metrics) {
        self.sample = sample
    }

    /// 采样一次并渲染成当前状态栏图标。
    func tick() -> NSImage {
        let m = sample()
        return StatusImage.makeStatusImage(
            downloadBps: m.downloadBps,
            uploadBps:   m.uploadBps,
            cpuPercent:  Int(m.cpuUsage),
            memPercent:  Int(m.memoryUsage)
        )
    }
}
