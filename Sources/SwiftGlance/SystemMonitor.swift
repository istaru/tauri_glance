import Foundation
import Darwin

/// 一次采样的全部指标。
struct Metrics {
    var cpuUsage: Double      // 0–100 (%)
    var memoryUsage: Double   // 0–100 (%)
    var memoryUsed: UInt64    // bytes
    var memoryTotal: UInt64   // bytes
    var downloadBps: Double   // bytes/s
    var uploadBps: Double     // bytes/s
}

/// 系统数据采集（§3，采用原生 API 法 B）。
/// 行为对齐命令法 A：内存口径 = active + wired + compressor，网络仅统计 en* 物理接口。
final class SystemMonitor {

    // 启动时读一次，之后不变。
    private let memoryTotal: UInt64
    private let pageSize: UInt64

    // CPU 差分用：上一次的每核 tick。
    private var prevCPUTicks: [(user: UInt32, system: UInt32, nice: UInt32, idle: UInt32)] = []

    // 网络差分用：上一次的累计字节（0 表示尚无历史）。
    private var previousRxBytes: UInt64 = 0
    private var previousTxBytes: UInt64 = 0
    private var hasNetHistory = false

    init() {
        var total: UInt64 = 0
        var sz = MemoryLayout<UInt64>.size
        sysctlbyname("hw.memsize", &total, &sz, nil, 0)
        memoryTotal = total
        pageSize = UInt64(vm_kernel_page_size)
    }

    // MARK: - 对外入口

    func sample() -> Metrics {
        let (used, total, memPct) = readMemory()
        let (rx, tx) = readNetworkSpeed()
        return Metrics(
            cpuUsage: readCPUUsage(),
            memoryUsage: memPct,
            memoryUsed: used,
            memoryTotal: total,
            downloadBps: rx,
            uploadBps: tx
        )
    }

    // MARK: - 3.1 CPU（host_processor_info / PROCESSOR_CPU_LOAD_INFO）

    private func readCPUUsage() -> Double {
        var cpuCount: natural_t = 0
        var infoArray: processor_info_array_t?
        var infoCount: mach_msg_type_number_t = 0

        let result = host_processor_info(
            mach_host_self(),
            PROCESSOR_CPU_LOAD_INFO,
            &cpuCount,
            &infoArray,
            &infoCount
        )
        guard result == KERN_SUCCESS, let infoArray = infoArray else {
            return 0
        }
        // 函数退出时释放内核分配的内存。
        defer {
            let size = vm_size_t(UInt(infoCount) * UInt(MemoryLayout<integer_t>.stride))
            vm_deallocate(mach_task_self_, vm_address_t(bitPattern: infoArray), size)
        }

        // 读出本次每核 tick。
        var current: [(user: UInt32, system: UInt32, nice: UInt32, idle: UInt32)] = []
        current.reserveCapacity(Int(cpuCount))
        for i in 0..<Int(cpuCount) {
            let base = i * Int(CPU_STATE_MAX)
            let user   = UInt32(bitPattern: infoArray[base + Int(CPU_STATE_USER)])
            let system = UInt32(bitPattern: infoArray[base + Int(CPU_STATE_SYSTEM)])
            let nice   = UInt32(bitPattern: infoArray[base + Int(CPU_STATE_NICE)])
            let idle   = UInt32(bitPattern: infoArray[base + Int(CPU_STATE_IDLE)])
            current.append((user, system, nice, idle))
        }

        // 首秒无历史 → 返回 0。
        guard prevCPUTicks.count == current.count, !prevCPUTicks.isEmpty else {
            prevCPUTicks = current
            return 0
        }

        var sum = 0.0
        for i in 0..<current.count {
            let dUser   = Double(current[i].user   &- prevCPUTicks[i].user)
            let dSystem = Double(current[i].system &- prevCPUTicks[i].system)
            let dNice   = Double(current[i].nice   &- prevCPUTicks[i].nice)
            let dIdle   = Double(current[i].idle   &- prevCPUTicks[i].idle)
            let busy = dUser + dSystem + dNice
            let totalTicks = busy + dIdle
            if totalTicks > 0 {
                sum += busy / totalTicks * 100.0
            }
        }
        prevCPUTicks = current

        let avg = sum / Double(current.count)
        return min(max(avg, 0), 100)
    }

    // MARK: - 3.2 内存（host_statistics64 / HOST_VM_INFO64）

    private func readMemory() -> (used: UInt64, total: UInt64, percent: Double) {
        var stats = vm_statistics64_data_t()
        var count = mach_msg_type_number_t(
            MemoryLayout<vm_statistics64_data_t>.stride / MemoryLayout<integer_t>.stride
        )
        let result = withUnsafeMutablePointer(to: &stats) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count)
            }
        }
        guard result == KERN_SUCCESS, memoryTotal > 0 else {
            return (0, memoryTotal, 0)
        }

        let used = (UInt64(stats.active_count)
                    + UInt64(stats.wire_count)
                    + UInt64(stats.compressor_page_count)) * pageSize

        let percent = min(max(Double(used) / Double(memoryTotal) * 100.0, 0), 100)
        return (used, memoryTotal, percent)
    }

    // MARK: - 3.3 网络（getifaddrs + AF_LINK）

    private func readNetworkSpeed() -> (downloadBps: Double, uploadBps: Double) {
        var rx: UInt64 = 0
        var tx: UInt64 = 0

        var ifaddrPtr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddrPtr) == 0, let first = ifaddrPtr else {
            return (0, 0)
        }
        defer { freeifaddrs(ifaddrPtr) }

        var ptr: UnsafeMutablePointer<ifaddrs>? = first
        while let cur = ptr {
            defer { ptr = cur.pointee.ifa_next }

            // 只看 AF_LINK 层。
            guard let addr = cur.pointee.ifa_addr,
                  addr.pointee.sa_family == UInt8(AF_LINK) else { continue }

            // 只统计 en* 物理接口（以太网/Wi-Fi），跳过 lo0 等。
            let name = String(cString: cur.pointee.ifa_name)
            guard name.hasPrefix("en") else { continue }

            guard let data = cur.pointee.ifa_data?
                    .assumingMemoryBound(to: if_data.self) else { continue }
            rx &+= UInt64(data.pointee.ifi_ibytes)
            tx &+= UInt64(data.pointee.ifi_obytes)
        }

        // 首次采样无历史 → 速度 0。
        guard hasNetHistory else {
            previousRxBytes = rx
            previousTxBytes = tx
            hasNetHistory = true
            return (0, 0)
        }

        // 处理计数回绕：差为负时按 0。
        let dRx = rx >= previousRxBytes ? rx - previousRxBytes : 0
        let dTx = tx >= previousTxBytes ? tx - previousTxBytes : 0
        previousRxBytes = rx
        previousTxBytes = tx

        let down = max(0, Double(dRx))
        let up   = max(0, Double(dTx))
        return (down, up)
    }
}
