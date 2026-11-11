//! Rounded rectangles. Pixels are `0xAARRGGBB`. A depth-24 visual ignores
//! the high byte; a transparent visual uses it, so opaque paint carries
//! `0xFF` and the corners outside the radius stay `0`.

use crate::Rect;

pub fn pack(rgb: u32) -> u32 {
    0xFF00_0000 | (rgb & 0x00FF_FFFF)
}

/// `text_parts` of 255 come from `text`, the rest from `background`.
pub fn mix(text: u32, background: u32, text_parts: u32) -> u32 {
    let text_parts = text_parts.min(255);
    let channel = |shift: u32| {
        let from_text = (text >> shift) & 0xff;
        let from_background = (background >> shift) & 0xff;
        (from_text * text_parts + from_background * (255 - text_parts)) / 255
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

pub fn fill_round(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    rect: Rect,
    radius: f32,
    color: u32,
) {
    paint(buffer, (width, height), rect, radius, color, false, None);
}

/// `rect` painted only where `clip` is opaque, so a highlight that meets the
/// window edge keeps the window's rounded corner.
pub fn fill_clipped(
    buffer: &mut [u32],
    size: (u32, u32),
    rect: Rect,
    radii: [f32; 4],
    clip: (Rect, f32),
    color: u32,
) {
    paint_corners(buffer, size, rect, radii, color, false, Some(clip));
}

pub fn stroke_round(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    rect: Rect,
    radius: f32,
    color: u32,
) {
    paint(buffer, (width, height), rect, radius, color, true, None);
}

/// The largest window corner that still fits the highlight. A bigger radius
/// would outgrow the highlight, so the two corners could no longer share a
/// center. With no highlight, the limit is half the window's shorter side.
pub fn capped_radius(requested: f32, frame: Rect, highlight: Option<Rect>) -> f32 {
    let window_limit = (frame.w.min(frame.h) * 0.5).max(0.0);
    let limit = match highlight {
        Some(inner) => window_limit.min((inner.w.min(inner.h) * 0.5).max(0.0)),
        None => window_limit,
    };
    requested.max(0.0).min(limit)
}

/// Highlight corner radii, top-left then clockwise. Every corner uses the
/// window radius minus the nearest gap, so the curves share a center and the
/// gap stays the same into the corner.
pub fn highlight_radii(frame: Rect, radius: f32, inner: Rect) -> [f32; 4] {
    let gap = inset(frame, inner).into_iter().fold(f32::MAX, f32::min);
    let gap = if gap.is_finite() { gap.max(0.0) } else { 0.0 };
    let limit = (inner.w.min(inner.h) * 0.5).max(0.0);
    let radius = (radius - gap).max(0.0).min(limit);
    [radius; 4]
}

fn inset(frame: Rect, inner: Rect) -> [f32; 4] {
    [
        (inner.x - frame.x).max(0.0),
        (inner.y - frame.y).max(0.0),
        (frame.x + frame.w - (inner.x + inner.w)).max(0.0),
        (frame.y + frame.h - (inner.y + inner.h)).max(0.0),
    ]
}

fn paint(
    buffer: &mut [u32],
    size: (u32, u32),
    rect: Rect,
    radius: f32,
    color: u32,
    stroke: bool,
    clip: Option<(Rect, f32)>,
) {
    paint_corners(buffer, size, rect, [radius; 4], color, stroke, clip);
}

fn paint_corners(
    buffer: &mut [u32],
    size: (u32, u32),
    rect: Rect,
    radii: [f32; 4],
    color: u32,
    stroke: bool,
    clip: Option<(Rect, f32)>,
) {
    let (width, height) = size;
    if rect.w <= 0.0 || rect.h <= 0.0 || width == 0 || height == 0 {
        return;
    }
    let x0 = rect.x.floor().max(0.0) as u32;
    let y0 = rect.y.floor().max(0.0) as u32;
    let x1 = ((rect.x + rect.w).ceil() as u32).min(width);
    let y1 = ((rect.y + rect.h).ceil() as u32).min(height);
    // The stroke sits one pixel inside the edge. A ring on the edge itself
    // fades out on the squircle, so the corner looks like it ate the line.
    let stroke_outer = inset_rect(rect, 0.5);
    let stroke_inner = inset_rect(rect, 1.5);
    let stroke_outer_radii = radii.map(|radius| (radius - 0.5).max(0.0));
    let stroke_inner_radii = radii.map(|radius| (radius - 1.5).max(0.0));
    for y in y0..y1 {
        for x in x0..x1 {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let mut amount = if stroke {
                (coverage(px, py, stroke_outer, stroke_outer_radii)
                    - coverage(px, py, stroke_inner, stroke_inner_radii))
                .clamp(0.0, 1.0)
            } else {
                coverage(px, py, rect, radii)
            };
            if let Some((clip, clip_radius)) = clip {
                amount *= coverage(px, py, clip, [clip_radius; 4]);
            }
            if amount <= 0.0 {
                continue;
            }
            let index = y as usize * width as usize + x as usize;
            buffer[index] = blend(buffer[index], color, amount);
        }
    }
}

/// A squircle: the superellipse `|x/r|⁴ + |y/r|⁴ = 1`. Curvature falls to
/// zero where the corner meets the straight edge, which a quarter circle
/// does not. `radii` is top-left, top-right, bottom-right, bottom-left.
fn coverage(px: f32, py: f32, rect: Rect, radii: [f32; 4]) -> f32 {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return 0.0;
    }
    let limit = rect.w.min(rect.h) * 0.5;
    let radii = radii.map(|radius| radius.max(0.0).min(limit));
    let (radius, center_x, center_y) = if px < rect.x + rect.w * 0.5 {
        if py < rect.y + rect.h * 0.5 {
            (radii[0], rect.x + radii[0], rect.y + radii[0])
        } else {
            (radii[3], rect.x + radii[3], rect.y + rect.h - radii[3])
        }
    } else if py < rect.y + rect.h * 0.5 {
        (radii[1], rect.x + rect.w - radii[1], rect.y + radii[1])
    } else {
        (
            radii[2],
            rect.x + rect.w - radii[2],
            rect.y + rect.h - radii[2],
        )
    };
    let in_corner = radius > 0.0
        && (px - center_x).abs() <= radius + 1.0
        && (py - center_y).abs() <= radius + 1.0
        && ((px < rect.x + radius) || (px > rect.x + rect.w - radius))
        && ((py < rect.y + radius) || (py > rect.y + rect.h - radius));
    if in_corner {
        let nx = (px - center_x) / radius;
        let ny = (py - center_y) / radius;
        let power = (nx.abs().powf(4.0) + ny.abs().powf(4.0)).powf(0.25);
        return (0.5 - (power - 1.0) * radius).clamp(0.0_f32, 1.0);
    }
    if px >= rect.x && py >= rect.y && px < rect.x + rect.w && py < rect.y + rect.h {
        return 1.0;
    }
    let dx = if px < rect.x {
        rect.x - px
    } else if px > rect.x + rect.w {
        px - (rect.x + rect.w)
    } else {
        0.0
    };
    let dy = if py < rect.y {
        rect.y - py
    } else if py > rect.y + rect.h {
        py - (rect.y + rect.h)
    } else {
        0.0
    };
    (0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0_f32, 1.0)
}

fn inset_rect(rect: Rect, by: f32) -> Rect {
    Rect {
        x: rect.x + by,
        y: rect.y + by,
        w: (rect.w - by * 2.0).max(0.0),
        h: (rect.h - by * 2.0).max(0.0),
    }
}

/// Straight-alpha pixels, for a compositor that stores premultiplied ARGB
/// (an X11 Render visual). Opaque pixels stay put. A transparent pixel
/// drops its color, so a rounded corner does not leave a bright fringe.
pub fn premultiply(buffer: &mut [u32]) {
    for pixel in buffer.iter_mut() {
        let alpha = *pixel >> 24;
        if alpha == 0 {
            *pixel = 0;
            continue;
        }
        if alpha == 255 {
            continue;
        }
        let channel = |shift: u32| {
            let value = (*pixel >> shift) & 0xff;
            (value * alpha + 127) / 255
        };
        *pixel = (alpha << 24) | (channel(16) << 16) | (channel(8) << 8) | channel(0);
    }
}

fn blend(dst: u32, src: u32, amount: f32) -> u32 {
    let src_a = ((src >> 24) as f32 * amount) as u32;
    if src_a == 0 {
        return dst;
    }
    let dst_a = dst >> 24;
    let inverse = 255 - src_a.min(255);
    let channel = |shift: u32| {
        let source = (src >> shift) & 0xff;
        let destination = (dst >> shift) & 0xff;
        (source * src_a + destination * inverse) / 255
    };
    let alpha = src_a + dst_a * inverse / 255;
    (alpha << 24) | (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_fill_covers_the_rect_and_leaves_the_corner_of_a_round_one() {
        let mut buffer = [0_u32; 16];
        fill_round(
            &mut buffer,
            4,
            4,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 4.0,
                h: 4.0,
            },
            0.0,
            pack(0xFFFFFF),
        );
        assert_eq!(buffer[0], pack(0xFFFFFF));
        buffer.fill(0);
        fill_round(
            &mut buffer,
            4,
            4,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 4.0,
                h: 4.0,
            },
            3.0,
            pack(0xFFFFFF),
        );
        assert!(
            buffer[0] >> 24 < buffer[5] >> 24,
            "the corner is softer than the interior"
        );
        assert_eq!(buffer[5], pack(0xFFFFFF));
    }

    #[test]
    fn the_highlight_shares_the_window_center_and_the_radius_stops_there() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 40.0,
        };
        let highlight = Rect {
            x: 4.0,
            y: 4.0,
            w: 60.0,
            h: 16.0,
        };
        assert_eq!(capped_radius(32.0, frame, Some(highlight)), 8.0);
        // Nearest gap is 4, so every corner is 8 − 4 and the centers match.
        assert_eq!(highlight_radii(frame, 8.0, highlight), [4.0; 4]);
        let flush = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 40.0,
        };
        assert_eq!(highlight_radii(frame, 8.0, flush), [8.0; 4]);
    }

    #[test]
    fn the_stroke_stays_opaque_on_the_corner() {
        let mut buffer = vec![0_u32; 24 * 24];
        let shape = Rect {
            x: 0.0,
            y: 0.0,
            w: 24.0,
            h: 24.0,
        };
        fill_round(&mut buffer, 24, 24, shape, 8.0, pack(0xFFFFFF));
        stroke_round(&mut buffer, 24, 24, shape, 8.0, pack(0x000000));
        let at = |x: usize, y: usize| buffer[y * 24 + x];
        let edge = at(12, 1);
        let corner = at(2, 2);
        assert!(edge >> 24 > 200, "edge stroke alpha {}", edge >> 24);
        assert!(corner >> 24 > 200, "corner stroke alpha {}", corner >> 24);
    }

    #[test]
    fn premultiply_scales_color_by_alpha_and_clears_a_transparent_pixel() {
        let mut buffer = [0x80FF_0000, 0x0000_00FF, 0xFFFF_FFFF];
        premultiply(&mut buffer);
        assert_eq!(buffer[0], 0x8080_0000);
        assert_eq!(buffer[1], 0);
        assert_eq!(buffer[2], 0xFFFF_FFFF);
    }
}
