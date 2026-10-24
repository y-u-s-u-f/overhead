// Draws the app icon: `swift assets/icon.swift <output.png>` (1024×1024).
import AppKit

let size: CGFloat = 1024
let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon.png"

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let ctx = NSGraphicsContext.current!.cgContext

// Standard macOS icon grid: an 824pt rounded square centered on the canvas.
let tile = CGRect(x: 100, y: 100, width: 824, height: 824)
let tilePath = NSBezierPath(roundedRect: tile, xRadius: 185, yRadius: 185)

ctx.saveGState()
tilePath.addClip()
NSGradient(
    starting: NSColor(srgbRed: 0.16, green: 0.17, blue: 0.20, alpha: 1),
    ending: NSColor(srgbRed: 0.05, green: 0.055, blue: 0.07, alpha: 1))!
    .draw(in: tile, angle: -90)
ctx.restoreGState()

let center = CGPoint(x: size / 2, y: size / 2)

// The zone.
let ring = NSBezierPath(ovalIn: CGRect(x: center.x - 270, y: center.y - 270, width: 540, height: 540))
ring.lineWidth = 12
NSColor(white: 1, alpha: 0.16).setStroke()
ring.stroke()

// Home.
NSColor(white: 1, alpha: 0.85).setFill()
NSBezierPath(ovalIn: CGRect(x: center.x - 20, y: center.y - 20, width: 40, height: 40)).fill()

// The aircraft: an amber arrowhead crossing the zone, heading northeast.
ctx.saveGState()
ctx.translateBy(x: center.x + 112, y: center.y + 112)
ctx.rotate(by: -.pi / 4)
let s: CGFloat = 108
let arrow = NSBezierPath()
arrow.move(to: CGPoint(x: 0, y: s))
arrow.line(to: CGPoint(x: 0.72 * s, y: -0.8 * s))
arrow.line(to: CGPoint(x: 0, y: -0.42 * s))
arrow.line(to: CGPoint(x: -0.72 * s, y: -0.8 * s))
arrow.close()
NSColor(srgbRed: 0.98, green: 0.65, blue: 0.16, alpha: 1).setFill()
arrow.fill()
ctx.restoreGState()

try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
