import AppKit

// 生成「看一眼」应用图标主图（1024×1024 PNG）。
// 设计：圆角方形 + 蓝紫渐变背景 + 白色半圆仪表盘 + 指针（指向约 70%）。
// 用法：swift make_icon.swift <输出路径.png>

let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon_1024.png"
let S: CGFloat = 1024

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: Int(S), pixelsHigh: Int(S),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0
)!

NSGraphicsContext.saveGraphicsState()
let gctx = NSGraphicsContext(bitmapImageRep: rep)!
NSGraphicsContext.current = gctx
let ctx = gctx.cgContext

func deg(_ d: CGFloat) -> CGFloat { d * .pi / 180 }
func pt(_ center: CGPoint, _ r: CGFloat, _ angleDeg: CGFloat) -> CGPoint {
    CGPoint(x: center.x + r * cos(deg(angleDeg)), y: center.y + r * sin(deg(angleDeg)))
}

// ---- 圆角方形背景（留白内缩，贴合 macOS 图标网格）----
let inset: CGFloat = 96
let rect = CGRect(x: inset, y: inset, width: S - inset * 2, height: S - inset * 2)
let corner: CGFloat = rect.width * 0.2237
let bgPath = NSBezierPath(roundedRect: rect, xRadius: corner, yRadius: corner)

// 阴影
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -18), blur: 48,
              color: NSColor(white: 0, alpha: 0.28).cgColor)
NSColor.white.setFill()
bgPath.fill()
ctx.restoreGState()

// 渐变填充（上 #6366F1 → 下 #4338CA）
ctx.saveGState()
bgPath.addClip()
let top = NSColor(calibratedRed: 0.40, green: 0.40, blue: 0.95, alpha: 1)   // indigo-500
let bot = NSColor(calibratedRed: 0.26, green: 0.22, blue: 0.79, alpha: 1)   // indigo-700
let grad = NSGradient(starting: top, ending: bot)!
grad.draw(in: bgPath, angle: -90)
// 顶部高光
let glossTop = NSColor(white: 1, alpha: 0.16)
let glossBot = NSColor(white: 1, alpha: 0.0)
let gloss = NSGradient(starting: glossTop, ending: glossBot)!
let glossRect = CGRect(x: rect.minX, y: rect.midY, width: rect.width, height: rect.height / 2)
gloss.draw(in: glossRect, angle: -90)
ctx.restoreGState()

// ---- 仪表盘 ----
let center = CGPoint(x: S / 2, y: S * 0.46)
let R: CGFloat = 250
let lineW: CGFloat = 60
let startDeg: CGFloat = 215    // 左下
let endDeg: CGFloat = -35      // 右下（经过正上方 90°）
let sweep = startDeg - endDeg  // 250°
let valueFraction: CGFloat = 0.70
let valueDeg = startDeg - sweep * valueFraction   // ≈ 40°

func arcPath(from a0: CGFloat, to a1: CGFloat) -> NSBezierPath {
    let p = NSBezierPath()
    let steps = 120
    for i in 0...steps {
        let a = a0 + (a1 - a0) * CGFloat(i) / CGFloat(steps)
        let point = pt(center, R, a)
        if i == 0 { p.move(to: point) } else { p.line(to: point) }
    }
    return p
}

// 背景轨道（半透明白）
let track = arcPath(from: startDeg, to: endDeg)
track.lineWidth = lineW
track.lineCapStyle = .round
NSColor(white: 1, alpha: 0.30).setStroke()
track.stroke()

// 数值轨道（实白，0 → 70%）
let value = arcPath(from: startDeg, to: valueDeg)
value.lineWidth = lineW
value.lineCapStyle = .round
NSColor.white.setStroke()
value.stroke()

// 刻度（短白线）
for f in stride(from: 0.0 as CGFloat, through: 1.0, by: 0.25) {
    let a = startDeg - sweep * f
    let p = NSBezierPath()
    p.move(to: pt(center, R - lineW / 2 - 18, a))
    p.line(to: pt(center, R - lineW / 2 - 50, a))
    p.lineWidth = 12
    p.lineCapStyle = .round
    NSColor(white: 1, alpha: 0.55).setStroke()
    p.stroke()
}

// 指针（从中心指向 70% 处，锥形）
let tip = pt(center, R - 36, valueDeg)
let baseL = pt(center, 40, valueDeg + 90)
let baseR = pt(center, 40, valueDeg - 90)
let needle = NSBezierPath()
needle.move(to: tip)
needle.line(to: baseL)
needle.line(to: baseR)
needle.close()
NSColor.white.setFill()
needle.fill()

// 中心轴
let hubR: CGFloat = 58
let hub = NSBezierPath(ovalIn: CGRect(x: center.x - hubR, y: center.y - hubR, width: hubR * 2, height: hubR * 2))
NSColor.white.setFill()
hub.fill()
let hubInnerR: CGFloat = 26
let hubInner = NSBezierPath(ovalIn: CGRect(x: center.x - hubInnerR, y: center.y - hubInnerR, width: hubInnerR * 2, height: hubInnerR * 2))
NSColor(calibratedRed: 0.33, green: 0.30, blue: 0.86, alpha: 1).setFill()
hubInner.fill()

NSGraphicsContext.restoreGraphicsState()

guard let data = rep.representation(using: .png, properties: [:]) else {
    FileHandle.standardError.write("PNG 编码失败\n".data(using: .utf8)!)
    exit(1)
}
try! data.write(to: URL(fileURLWithPath: outPath))
print("已生成: \(outPath)")
