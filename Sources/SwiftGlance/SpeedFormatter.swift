import Foundation

/// 网速格式化阶梯（CLAUDE.md §4）。
///
/// 小 interface、深 implementation：输入 bytes/s，输出永远 1–2 位数字 + 单位，
/// 跨档用 `.X` 小数表示。纯函数，不依赖 AppKit——格式化逻辑的 test surface 就是它自己。
enum SpeedFormatter {

    /// 把 bytes/s 格式化成 (数字, 单位) 两段。
    /// 数字始终 ≤ 2 位；跨档边界（100B、100K、100M）用 `.1`–`.9` 小数，不出现 `.0`。
    static func format(_ bps: Double) -> (num: String, unit: String) {
        let KB: Double = 1024
        let MB: Double = 1024 * KB
        let GB: Double = 1024 * MB
        switch bps {
        case ..<100:          return ("\(Int(bps))", "B")
        case ..<KB:           return (".\(fraction(bps, KB))", "K")
        case ..<(100 * KB):   return ("\(Int(bps / KB))", "K")
        case ..<MB:           return (".\(fraction(bps, MB))", "M")
        case ..<(100 * MB):   return ("\(Int(bps / MB))", "M")
        case ..<GB:           return (".\(fraction(bps, GB))", "G")
        default:              return ("\(min(Int(bps / GB), 99))", "G")
        }
    }

    /// 跨档小数位：保证始终落在 1–9，不出现 `.0`。
    private static func fraction(_ bps: Double, _ nextUnit: Double) -> Int {
        max(1, min(9, Int(bps / nextUnit * 10)))
    }
}
