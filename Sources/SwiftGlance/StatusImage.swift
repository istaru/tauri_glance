import AppKit

enum StatusImage {

    private static let font = NSFont.monospacedDigitSystemFont(ofSize: 8, weight: .bold)
    private static let attrs: [NSAttributedString.Key: Any] = {
        let para = NSMutableParagraphStyle()
        para.alignment = .center
        return [
            .font: font,
            .foregroundColor: NSColor.black,
            .paragraphStyle: para
        ]
    }()
    private static let colGap: CGFloat = 1
    private static let lineH: CGFloat = NSAttributedString(string: "↓999K/s", attributes: attrs).size().height
    private static let colW: CGFloat  = NSAttributedString(string: "↓999K/s", attributes: attrs).size().width
    private static let imgW: CGFloat  = colW * 2 + colGap
    private static let imgH: CGFloat  = lineH * 2 + 2

    /// §5 速度格式化：输入 KB/s，整数显示，封顶 999。
    static func speedLabel(_ kbps: Double) -> String {
        if kbps < 1024 {
            return "\(min(Int(kbps), 999))K/s"
        } else {
            return "\(min(Int(kbps / 1024), 999))M/s"
        }
    }

    /// §4 自绘两排两列模板图标（直接复用规格中已验证的实现）。
    static func makeStatusImage(
        down: String,
        up: String,
        cpuVal: String,
        memVal: String,
        cpuShort: String = "C",
        memShort: String = "M"
    ) -> NSImage {
        let cpuLabel = "\(cpuShort) \(cpuVal)%"
        let memLabel = "\(memShort) \(memVal)%"
        let row1Y = lineH + 2

        let img = NSImage(size: NSSize(width: imgW, height: imgH), flipped: false) { _ in
            NSAttributedString(string: down, attributes: attrs)
                .draw(in: CGRect(x: 0, y: row1Y, width: colW, height: lineH))
            NSAttributedString(string: up, attributes: attrs)
                .draw(in: CGRect(x: colW + colGap, y: row1Y, width: colW, height: lineH))
            NSAttributedString(string: cpuLabel, attributes: attrs)
                .draw(in: CGRect(x: 0, y: 0, width: colW, height: lineH))
            NSAttributedString(string: memLabel, attributes: attrs)
                .draw(in: CGRect(x: colW + colGap, y: 0, width: colW, height: lineH))
            return true
        }
        img.isTemplate = true
        return img
    }
}
