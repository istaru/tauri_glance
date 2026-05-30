import AppKit

enum StatusImage {

    private static let font = NSFont.monospacedDigitSystemFont(ofSize: 8, weight: .bold)

    private static let lAttrs: [NSAttributedString.Key: Any] = {
        let para = NSMutableParagraphStyle()
        para.alignment = .left
        return [.font: font, .foregroundColor: NSColor.black, .paragraphStyle: para]
    }()

    private static let rAttrs: [NSAttributedString.Key: Any] = {
        let para = NSMutableParagraphStyle()
        para.alignment = .right
        return [.font: font, .foregroundColor: NSColor.black, .paragraphStyle: para]
    }()

    private static let colGap: CGFloat  = 3
    private static let sidePad: CGFloat = 2

    // 符号列宽：取所有符号中最宽的
    private static let symbolW: CGFloat = ["↓", "↑", "c", "m"].map {
        NSAttributedString(string: $0, attributes: lAttrs).size().width
    }.max()!

    // 数字列宽：最宽为两位整数"99"或小数".9"，取较大者
    private static let numberW: CGFloat = [".9", "99"].map {
        NSAttributedString(string: $0, attributes: lAttrs).size().width
    }.max()!

    // 单位列宽：取 B / K / M / G / % 中最宽的
    private static let unitW: CGFloat = ["B", "K", "M", "G", "%"].map {
        NSAttributedString(string: $0, attributes: lAttrs).size().width
    }.max()!

    private static let mainColW: CGFloat = symbolW + numberW + unitW
    private static let lineH: CGFloat =
        NSAttributedString(string: "↓", attributes: lAttrs).size().height
    private static let imgW: CGFloat = sidePad + mainColW + colGap + mainColW + sidePad
    private static let imgH: CGFloat = lineH * 2 + 2

    private static func speedParts(_ bps: Double) -> (num: String, unit: String) {
        let KB: Double = 1024
        let MB: Double = 1024 * KB
        let GB: Double = 1024 * MB
        switch bps {
        case ..<100:          return ("\(Int(bps))", "B")
        case ..<KB:           return (".\(max(1, min(9, Int(bps / KB * 10))))", "K")
        case ..<(100 * KB):   return ("\(Int(bps / KB))", "K")
        case ..<MB:           return (".\(max(1, min(9, Int(bps / MB * 10))))", "M")
        case ..<(100 * MB):   return ("\(Int(bps / MB))", "M")
        case ..<GB:           return (".\(max(1, min(9, Int(bps / GB * 10))))", "G")
        default:              return ("\(min(Int(bps / GB), 99))", "G")
        }
    }

    static func makeStatusImage(
        downloadBps: Double,
        uploadBps: Double,
        cpuPercent: Int,
        memPercent: Int
    ) -> NSImage {
        let (downNum, downUnit) = speedParts(downloadBps)
        let (upNum,   upUnit)   = speedParts(uploadBps)
        let cpuNum = "\(cpuPercent)"
        let memNum = "\(memPercent)"

        let row1Y  = lineH + 2
        let leftX  = sidePad
        let rightX = sidePad + mainColW + colGap

        func drawCell(sym: String, num: String, unit: String, x: CGFloat, y: CGFloat) {
            NSAttributedString(string: sym,  attributes: lAttrs)
                .draw(in: CGRect(x: x,                        y: y, width: symbolW, height: lineH))
            NSAttributedString(string: num,  attributes: rAttrs)
                .draw(in: CGRect(x: x + symbolW,              y: y, width: numberW, height: lineH))
            NSAttributedString(string: unit, attributes: lAttrs)
                .draw(in: CGRect(x: x + symbolW + numberW,    y: y, width: unitW,   height: lineH))
        }

        let img = NSImage(size: NSSize(width: imgW, height: imgH), flipped: false) { _ in
            drawCell(sym: "↓", num: downNum, unit: downUnit, x: leftX,  y: row1Y)
            drawCell(sym: "c", num: cpuNum,  unit: "%",      x: leftX,  y: 0)
            drawCell(sym: "↑", num: upNum,   unit: upUnit,   x: rightX, y: row1Y)
            drawCell(sym: "m", num: memNum,  unit: "%",      x: rightX, y: 0)
            return true
        }
        img.isTemplate = true
        return img
    }
}
