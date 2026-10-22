// Build the Windows icons (icon.ico and the PNGs the bundle lists) from the
// same artwork as macOS. Windows does not round app icons either, so the
// square white source showed as a sharp white block on the taskbar.
// Windows tiles fill more of the canvas than macOS ones and use a smaller
// radius; the outside is transparent.
import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
let icons = root.appendingPathComponent("src-tauri/icons")
let source = icons.appendingPathComponent("app-icon-source.png")

guard let imageSource = CGImageSourceCreateWithURL(source as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(imageSource, 0, nil),
      image.width == image.height else {
    fatalError("The app icon source must be a readable square image")
}

func renderTile(_ size: Int) -> Data {
    guard let context = CGContext(data: nil, width: size, height: size,
                                  bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
        fatalError("Unable to create icon bitmap")
    }
    context.clear(CGRect(x: 0, y: 0, width: size, height: size))
    context.interpolationQuality = .high
    // About 3% margin, pixel-aligned so small sizes stay symmetric.
    let inset = CGFloat((Double(size) * 0.03).rounded())
    let tile = CGRect(x: inset, y: inset,
                      width: CGFloat(size) - inset * 2,
                      height: CGFloat(size) - inset * 2)
    let radius = tile.width * 0.18
    let shape = CGPath(roundedRect: tile, cornerWidth: radius, cornerHeight: radius, transform: nil)
    context.saveGState()
    context.addPath(shape)
    context.clip()
    context.draw(image, in: tile)
    context.restoreGState()
    if size >= 32 {
        // A faint edge so the white tile doesn't dissolve into a white
        // Explorer or Start background.
        let line = max(1, CGFloat(size) / 256)
        let edge = tile.insetBy(dx: line / 2, dy: line / 2)
        let edgeRadius = max(0, radius - line / 2)
        context.addPath(CGPath(roundedRect: edge, cornerWidth: edgeRadius, cornerHeight: edgeRadius, transform: nil))
        context.setStrokeColor(CGColor(gray: 0, alpha: 0.12))
        context.setLineWidth(line)
        context.strokePath()
    }
    guard let bitmap = context.makeImage() else { fatalError("Unable to render icon") }
    let data = NSMutableData()
    guard let destination = CGImageDestinationCreateWithData(data, UTType.png.identifier as CFString, 1, nil) else {
        fatalError("Unable to encode icon")
    }
    CGImageDestinationAddImage(destination, bitmap, nil)
    guard CGImageDestinationFinalize(destination) else { fatalError("Unable to encode icon") }
    return data as Data
}

// PNGs listed in the Windows bundle config (also used as the window icon).
for (name, size) in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128),
                     ("128x128@2x.png", 256), ("icon.png", 512)] {
    try renderTile(size).write(to: icons.appendingPathComponent(name), options: .atomic)
}

// icon.ico with PNG-compressed entries (supported since Vista; the NSIS
// installer already used them). 20 and 40 cover 125% and 250% scaling.
let icoSizes = [16, 20, 24, 32, 40, 48, 64, 256]
let images = icoSizes.map(renderTile)
var ico = Data()
func le16(_ v: Int) { var x = UInt16(v).littleEndian; ico.append(Data(bytes: &x, count: 2)) }
func le32(_ v: Int) { var x = UInt32(v).littleEndian; ico.append(Data(bytes: &x, count: 4)) }
le16(0); le16(1); le16(icoSizes.count)
var offset = 6 + 16 * icoSizes.count
for (size, png) in zip(icoSizes, images) {
    ico.append(UInt8(size >= 256 ? 0 : size))
    ico.append(UInt8(size >= 256 ? 0 : size))
    ico.append(0); ico.append(0)
    le16(1); le16(32)
    le32(png.count); le32(offset)
    offset += png.count
}
images.forEach { ico.append($0) }
try ico.write(to: icons.appendingPathComponent("icon.ico"), options: .atomic)
print("Generated Windows icons: rounded tile, sizes \(icoSizes)")
