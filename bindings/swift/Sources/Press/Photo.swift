// SPDX-License-Identifier: GPL-3.0-or-later
// A picture in colour: press's one exception to black and white, for what
// is a photograph by nature, such as a camera's picture. Offer it in black
// and white too.

import Foundation
import PressCore

/// A colour picture for a canvas to draw: rows of `width` pixels, three
/// bytes each, red, green and blue.
public struct Photo: Sendable {
    public let width: Int
    public let height: Int
    public let rgb: [UInt8]

    public init(rgb: [UInt8], width: Int, height: Int) {
        self.rgb = rgb
        self.width = width
        self.height = height
    }
}

extension GraphicsContext {
    /// Lay down a colour picture with its top left corner at a point. Like
    /// an image, it covers what was there, and lines and words are drawn
    /// over it.
    public func draw(_ photo: Photo, at point: CGPoint) {
        try? picture.photo(pixels(point.x), y: pixels(point.y), width: photo.width, height: photo.height, rgb: photo.rgb)
    }
}
