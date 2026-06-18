import Foundation

/// 把相邻两次 `RawCounters` 读数变成 `Metrics`——纯差分逻辑，不碰 syscall。
///
/// 持有差分历史（上一次每核 tick、上一次累计字节）。喂入脚本化的 `RawCounters`
/// 即可测：CPU 多核平均、网络回绕、首样本归零、内存百分比口径全在此处。
final class RateDiffer {

    private var prevCPU: [RawCounters.CPUTicks] = []
    private var prevRx: UInt64 = 0
    private var prevTx: UInt64 = 0
    private var hasNetHistory = false

    func diff(_ raw: RawCounters) -> Metrics {
        let (down, up) = networkSpeed(rx: raw.rxBytes, tx: raw.txBytes)
        return Metrics(
            cpuUsage: cpuUsage(raw.cpu),
            memoryUsage: memoryPercent(used: raw.memoryUsed, total: raw.memoryTotal),
            memoryUsed: raw.memoryUsed,
            memoryTotal: raw.memoryTotal,
            downloadBps: down,
            uploadBps: up
        )
    }

    // MARK: - CPU 差分（跨核平均，首样本无历史返回 0）

    private func cpuUsage(_ current: [RawCounters.CPUTicks]) -> Double {
        guard prevCPU.count == current.count, !prevCPU.isEmpty else {
            prevCPU = current
            return 0
        }

        var sum = 0.0
        for i in 0..<current.count {
            let dUser   = Double(current[i].user   &- prevCPU[i].user)
            let dSystem = Double(current[i].system &- prevCPU[i].system)
            let dNice   = Double(current[i].nice   &- prevCPU[i].nice)
            let dIdle   = Double(current[i].idle   &- prevCPU[i].idle)
            let busy = dUser + dSystem + dNice
            let totalTicks = busy + dIdle
            if totalTicks > 0 {
                sum += busy / totalTicks * 100.0
            }
        }
        prevCPU = current

        let avg = sum / Double(current.count)
        return min(max(avg, 0), 100)
    }

    // MARK: - 内存百分比（口径与活动监视器一致）

    private func memoryPercent(used: UInt64, total: UInt64) -> Double {
        guard total > 0 else { return 0 }
        return min(max(Double(used) / Double(total) * 100.0, 0), 100)
    }

    // MARK: - 网络差分（首样本归零，处理计数回绕）

    private func networkSpeed(rx: UInt64?, tx: UInt64?) -> (down: Double, up: Double) {
        // 本次读取失败 → 保留历史，速度按 0。
        guard let rx = rx, let tx = tx else { return (0, 0) }

        // 首次采样无历史 → 速度 0。
        guard hasNetHistory else {
            prevRx = rx
            prevTx = tx
            hasNetHistory = true
            return (0, 0)
        }

        // 处理计数回绕：差为负时按 0。
        let dRx = rx >= prevRx ? rx - prevRx : 0
        let dTx = tx >= prevTx ? tx - prevTx : 0
        prevRx = rx
        prevTx = tx

        return (max(0, Double(dRx)), max(0, Double(dTx)))
    }
}
