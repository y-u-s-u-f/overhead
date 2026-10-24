// Draws the disk image background: `swift assets/dmg-background.swift <output.png> <scale>`.
// The window is 660×400; the app sits at (170, 190) and Applications at (490, 190).
import AppKit

let args = CommandLine.arguments
let out = args.count > 1 ? args[1] : "background.png"
let scale = CGFloat(args.count > 2 ? Double(args[2])! : 1)
let width: CGFloat = 660, height: CGFloat = 400

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: Int(width * scale), pixelsHigh: Int(height * scale),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
rep.size = NSSize(width: width, height: height)
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let ctx = NSGraphicsContext.current!.cgContext

// Flip to top-left origin so coordinates match Finder's icon positions.
ctx.translateBy(x: 0, y: height)
ctx.scaleBy(x: 1, y: -1)

NSGradient(
    starting: NSColor(srgbRed: 0.975, green: 0.972, blue: 0.965, alpha: 1),
    ending: NSColor(srgbRed: 0.935, green: 0.93, blue: 0.92, alpha: 1))!
    .draw(in: CGRect(x: 0, y: 0, width: width, height: height), angle: 90)

// The flight path: a shallow arc from the app to Applications.
let start = CGPoint(x: 252, y: 178)
let control = CGPoint(x: 330, y: 112)
let end = CGPoint(x: 408, y: 178)

func point(_ t: CGFloat) -> CGPoint {
    let u = 1 - t
    return CGPoint(
        x: u * u * start.x + 2 * u * t * control.x + t * t * end.x,
        y: u * u * start.y + 2 * u * t * control.y + t * t * end.y)
}

func tangent(_ t: CGFloat) -> CGFloat {
    let dx = 2 * (1 - t) * (control.x - start.x) + 2 * t * (end.x - control.x)
    let dy = 2 * (1 - t) * (control.y - start.y) + 2 * t * (end.y - control.y)
    return atan2(dy, dx)
}

// The trail behind the plane, in dots that fade in as they near it.
let planeAt: CGFloat = 0.74
let dots = 13
for i in 0..<dots {
    let t = planeAt * CGFloat(i) / CGFloat(dots) + 0.02
    let p = point(t)
    let alpha = 0.10 + 0.32 * CGFloat(i) / CGFloat(dots)
    NSColor(white: 0.1, alpha: alpha).setFill()
    let r: CGFloat = 1.9
    NSBezierPath(ovalIn: CGRect(x: p.x - r, y: p.y - r, width: 2 * r, height: 2 * r)).fill()
}

// The plane, seen from above with its nose along +x.
ctx.saveGState()
let p = point(planeAt)
ctx.translateBy(x: p.x, y: p.y)
ctx.rotate(by: tangent(planeAt))
let plane = NSBezierPath()
plane.move(to: CGPoint(x: 15, y: 0))
plane.curve(to: CGPoint(x: 10, y: -2.2), controlPoint1: CGPoint(x: 15, y: -1.4), controlPoint2: CGPoint(x: 12.5, y: -2.2))
plane.line(to: CGPoint(x: 3, y: -2.2))
plane.line(to: CGPoint(x: -4, y: -14))
plane.line(to: CGPoint(x: -7.5, y: -14))
plane.line(to: CGPoint(x: -3, y: -2.2))
plane.line(to: CGPoint(x: -10, y: -2.2))
plane.line(to: CGPoint(x: -13, y: -7))
plane.line(to: CGPoint(x: -15.5, y: -7))
plane.line(to: CGPoint(x: -14, y: 0))
plane.line(to: CGPoint(x: -15.5, y: 7))
plane.line(to: CGPoint(x: -13, y: 7))
plane.line(to: CGPoint(x: -10, y: 2.2))
plane.line(to: CGPoint(x: -3, y: 2.2))
plane.line(to: CGPoint(x: -7.5, y: 14))
plane.line(to: CGPoint(x: -4, y: 14))
plane.line(to: CGPoint(x: 3, y: 2.2))
plane.line(to: CGPoint(x: 10, y: 2.2))
plane.curve(to: CGPoint(x: 15, y: 0), controlPoint1: CGPoint(x: 12.5, y: 2.2), controlPoint2: CGPoint(x: 15, y: 1.4))
plane.close()
NSColor(srgbRed: 0.96, green: 0.6, blue: 0.12, alpha: 1).setFill()
plane.fill()
ctx.restoreGState()

try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
