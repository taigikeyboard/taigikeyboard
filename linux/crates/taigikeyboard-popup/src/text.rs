//! The type the window draws, and the pixels of one page.
//!
//! The settings sample calls [`paint_arrangement`] with the same fonts, so
//! a row it shows is the window's row. The buffer stays straight alpha;
//! the window premultiplies when it hands the buffer to the compositor.

use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache};

use crate::face::{resolve_face, ResolvedFace};
use crate::layout::{Arrangement, Glyph, Measure, Role};
use crate::paint;
use crate::protocol::Chrome;
use crate::Rect;

pub struct Fonts {
    system: std::cell::RefCell<FontSystem>,
    cache: std::cell::RefCell<SwashCache>,
    hanji_family: std::cell::RefCell<String>,
    roman_family: std::cell::RefCell<String>,
    hanji_face: std::cell::RefCell<Option<ResolvedFace>>,
    roman_face: std::cell::RefCell<Option<ResolvedFace>>,
}

impl Fonts {
    pub fn load() -> Self {
        Self {
            system: std::cell::RefCell::new(FontSystem::new()),
            cache: std::cell::RefCell::new(SwashCache::new()),
            hanji_family: std::cell::RefCell::new(String::new()),
            roman_family: std::cell::RefCell::new(String::new()),
            hanji_face: std::cell::RefCell::new(None),
            roman_face: std::cell::RefCell::new(None),
        }
    }

    pub fn set_families(&self, hanji: &str, roman: &str) {
        if self.hanji_family.borrow().as_str() != hanji {
            *self.hanji_family.borrow_mut() = hanji.to_owned();
            *self.hanji_face.borrow_mut() = None;
        }
        if self.roman_family.borrow().as_str() != roman {
            *self.roman_family.borrow_mut() = roman.to_owned();
            *self.roman_face.borrow_mut() = None;
        }
    }

    fn resolved(&self, hanji: bool) -> ResolvedFace {
        let slot = if hanji {
            &self.hanji_face
        } else {
            &self.roman_face
        };
        if let Some(face) = slot.borrow().clone() {
            return face;
        }
        let requested = if hanji {
            self.hanji_family.borrow().clone()
        } else {
            self.roman_family.borrow().clone()
        };
        let face = resolve_face(self.system.borrow().db(), &requested);
        *slot.borrow_mut() = Some(face.clone());
        face
    }

    fn draw(&self, glyph: &Glyph, buffer: &mut [u32], width: u32, height: u32, color: Color) {
        if glyph.text.is_empty() || glyph.px <= 0.0 {
            return;
        }
        let resolved = self.resolved(glyph.hanji);
        let attrs = face(&resolved);
        let mut system = self.system.borrow_mut();
        let mut cache = self.cache.borrow_mut();
        let mut text = Buffer::new(&mut system, Metrics::new(glyph.px, glyph.h.max(glyph.px)));
        text.set_size(&mut system, Some(glyph.w.ceil()), Some(glyph.h.ceil()));
        text.set_text(&mut system, &glyph.text, &attrs, Shaping::Advanced);
        text.shape_until_scroll(&mut system, false);
        let origin_x = glyph.x.round() as i32;
        let origin_y = glyph.y.round() as i32;
        text.draw(&mut system, &mut cache, color, |x, y, w, h, pixel| {
            blit(
                buffer,
                (width, height),
                (origin_x + x, origin_y + y, w, h),
                pixel,
            );
        });
    }
}

/// Chrome, highlight and glyphs into `buffer`, which the caller has cleared.
/// `size` is the buffer, in the same pixels as `arrangement`.
pub fn paint_arrangement(
    fonts: &Fonts,
    chrome: &Chrome,
    arrangement: &Arrangement,
    scale: f32,
    buffer: &mut [u32],
    size: (u32, u32),
) {
    let (width, height) = size;
    if width == 0 || height == 0 {
        return;
    }
    fonts.set_families(&chrome.hanji_family, &chrome.roman_family);
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        w: width as f32,
        h: height as f32,
    };
    let radius = paint::capped_radius(chrome.radius as f32 * scale, frame, arrangement.highlight);
    paint::fill_round(
        buffer,
        width,
        height,
        frame,
        radius,
        paint::pack(chrome.background_color),
    );
    if let Some(rect) = arrangement.highlight {
        paint::fill_clipped(
            buffer,
            size,
            rect,
            paint::highlight_radii(frame, radius, rect),
            (frame, radius),
            paint::pack(chrome.selected_background_color),
        );
    }
    let palette = Palette::from_chrome(chrome);
    for glyph in &arrangement.glyphs {
        let selected = arrangement.highlight.is_some_and(|rect| {
            let x = glyph.x + glyph.w / 2.0;
            let y = glyph.y + glyph.h / 2.0;
            x >= rect.x && y >= rect.y && x < rect.x + rect.w && y < rect.y + rect.h
        });
        fonts.draw(
            glyph,
            buffer,
            width,
            height,
            palette.color(glyph.role, selected),
        );
    }
    paint::stroke_round(
        buffer,
        width,
        height,
        frame,
        radius,
        paint::pack(paint::mix(chrome.text_color, chrome.background_color, 80)),
    );
}

struct Palette {
    text: Color,
    annotation: Color,
    selected_text: Color,
    selected_annotation: Color,
    label: Color,
    arrow: Color,
}

impl Palette {
    fn from_chrome(chrome: &Chrome) -> Self {
        let text = color(chrome.text_color);
        let selected = color(chrome.selected_text_color);
        Self {
            text,
            annotation: color(paint::mix(chrome.text_color, chrome.background_color, 150)),
            selected_text: selected,
            selected_annotation: color(paint::mix(
                chrome.selected_text_color,
                chrome.selected_background_color,
                150,
            )),
            label: color(paint::mix(chrome.text_color, chrome.background_color, 110)),
            arrow: color(paint::mix(chrome.text_color, chrome.background_color, 160)),
        }
    }

    fn color(&self, role: Role, selected: bool) -> Color {
        match role {
            Role::Text => {
                if selected {
                    self.selected_text
                } else {
                    self.text
                }
            }
            Role::Annotation => {
                if selected {
                    self.selected_annotation
                } else {
                    self.annotation
                }
            }
            Role::Label => {
                if selected {
                    self.selected_text
                } else {
                    self.label
                }
            }
            Role::Arrow => self.arrow,
        }
    }
}

fn color(rgb: u32) -> Color {
    Color::rgb(
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

fn face(resolved: &ResolvedFace) -> Attrs<'_> {
    if resolved.family.is_empty() {
        Attrs::new()
    } else {
        Attrs::new()
            .family(Family::Name(&resolved.family))
            .weight(resolved.weight)
            .style(resolved.style)
            .stretch(resolved.stretch)
    }
}

impl Measure for Fonts {
    fn size(&self, text: &str, px: f32, hanji: bool) -> (f32, f32) {
        if text.is_empty() || px <= 0.0 {
            return (0.0, px.max(0.0));
        }
        let resolved = self.resolved(hanji);
        let attrs = face(&resolved);
        let mut system = self.system.borrow_mut();
        let metrics = Metrics::new(px, px);
        let mut buffer = Buffer::new(&mut system, metrics);
        buffer.set_size(&mut system, None, None);
        buffer.set_text(&mut system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut system, false);
        let mut width = 0.0_f32;
        let mut height = 0.0_f32;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += run.line_height;
        }
        (width.ceil(), height.max(px))
    }

    fn baseline(&self, text: &str, px: f32, hanji: bool) -> f32 {
        if text.is_empty() || px <= 0.0 {
            return 0.0;
        }
        let resolved = self.resolved(hanji);
        let attrs = face(&resolved);
        let mut system = self.system.borrow_mut();
        let mut buffer = Buffer::new(&mut system, Metrics::new(px, px));
        buffer.set_size(&mut system, None, None);
        buffer.set_text(&mut system, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut system, false);
        buffer
            .layout_runs()
            .next()
            .map(|run| run.line_y)
            .unwrap_or(px)
    }
}

fn blit(buffer: &mut [u32], size: (u32, u32), stamp: (i32, i32, u32, u32), pixel: Color) {
    let (width, height) = size;
    let (x, y, w, h) = stamp;
    let alpha = pixel.a();
    if alpha == 0 {
        return;
    }
    for row in 0..h {
        for column in 0..w {
            let px = x + column as i32;
            let py = y + row as i32;
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                continue;
            }
            let index = py as usize * width as usize + px as usize;
            buffer[index] = over(buffer[index], pixel);
        }
    }
}

fn over(dst: u32, src: Color) -> u32 {
    let alpha = u32::from(src.a());
    if alpha == 0 {
        return dst;
    }
    let inverse = 255 - alpha;
    let blend =
        |source: u8, destination: u32| (u32::from(source) * alpha + destination * inverse) / 255;
    let red = blend(src.r(), (dst >> 16) & 0xff);
    let green = blend(src.g(), (dst >> 8) & 0xff);
    let blue = blend(src.b(), dst & 0xff);
    let out_alpha = alpha + (dst >> 24) * inverse / 255;
    (out_alpha << 24) | (red << 16) | (green << 8) | blue
}
