import Foundation
import Darwin

/// 一次「原始累计计数器」快照——尚未差分的裸数据。
/// CPU 是每核 tick，网络是累计字节，内存是瞬时占用（无需差分）。
struct RawCounters: Equatable {
    struct CPUTicks: Equatable {
        var user: UInt32
        var system: UInt32
        var nice: UInt32
        var idle: UInt32
    }

    /// 每核 tick；空数组表示本次 CPU 读取失败。
    var cpu: [CPUTicks]
    /// 内存瞬时占用 / 总量（bytes）。
    var memoryUsed: UInt64
    var memoryTotal: UInt64
    /// 累计收发字节；nil 表示本次网络读取失败（差分时应保留历史、速度按 0）。
    var rxBytes: UInt64?
    var txBytes: UInt64?
}

/// 原始计数器的来源（port）。
/// 活体实现碰 syscall；测试实现喂脚本化读数，让 `RateDiffer` 的 seam 名副其实。
protocol CounterSource {
    func read() -> RawCounters
}

/// 活体 adapter：从 macOS 内核读原始计数器（§3 原生 API 法 B）。
/// 无差分状态——只负责读当前值；`hw.memsize` 与 `pageSize` 在 init 缓存一次。
final class LiveCounters: CounterSource {

    private let memoryTotal: UInt64
    private let pageSize: UInt64

    init() {
        var total: UInt64 = 0
        var sz = MemoryLayout<UInt64>.size
        sysctlbyname("hw.memsize", &total, &sz, nil, 0)
        memoryTotal = total
        pageSize = UInt64(vm_kernel_page_size)
    }

    func read() -> RawCounters {
        let (used, total) = readMemory()
        let net = readNetwork()
        return RawCounters(
            cpu: readCPUTicks(),
            memoryUsed: used,
            memoryTotal: total,
            rxBytes: net?.rx,
            txBytes: net?.tx
        )
    }

    // MARK: - 3.1 CPU（host_processor_info / PROCESSOR_CPU_LOAD_INFO）

    private func readCPUTicks() -> [RawCounters.CPUTicks] {
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
            return []
        }
        defer {
            let size = vm_size_t(UInt(infoCount) * UInt(MemoryLayout<integer_t>.stride))
            vm_deallocate(mach_task_self_, vm_address_t(bitPattern: infoArray), size)
        }

        var ticks: [RawCounters.CPUTicks] = []
        ticks.reserveCapacity(Int(cpuCount))
        for i in 0..<Int(cpuCount) {
            let base = i * Int(CPU_STATE_MAX)
            ticks.append(RawCounters.CPUTicks(
                user:   UInt32(bitPattern: infoArray[base + Int(CPU_STATE_USER)]),
                system: UInt32(bitPattern: infoArray[base + Int(CPU_STATE_SYSTEM)]),
                nice:   UInt32(bitPattern: infoArray[base + Int(CPU_STATE_NICE)]),
                idle:   UInt32(bitPattern: infoArray[base + Int(CPU_STATE_IDLE)])
            ))
        }
        return ticks
    }

    // MARK: - 3.2 内存（host_statistics64 / HOST_VM_INFO64）

    private func readMemory() -> (used: UInt64, total: UInt64) {
        var stats = vm_statistics64_data_t()
        var count = mach_msg_type_number_t(
            MemoryLayout<vm_statistics64_data_t>.stride / MemoryLayout<integer_t>.stride
        )
        let result = withUnsafeMutablePointer(to: &stats) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                host_statistics64(mach_host_self(), HOST_VM_INFO64, $0, &count)
            }
        }
        guard result == KERN_SUCCESS else {
            return (0, memoryTotal)
        }
        let used = (UInt64(stats.active_count)
                    + UInt64(stats.wire_count)
                    + UInt64(stats.compressor_page_count)) * pageSize
        return (used, memoryTotal)
    }

    // MARK: - 3.3 网络（getifaddrs + AF_LINK）

    private func readNetwork() -> (rx: UInt64, tx: UInt64)? {
        var ifaddrPtr: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&ifaddrPtr) == 0, let first = ifaddrPtr else {
            return nil
        }
        defer { freeifaddrs(ifaddrPtr) }

        var rx: UInt64 = 0
        var tx: UInt64 = 0
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
        return (rx, tx)
    }
}
