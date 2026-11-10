use tiny_skia::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub const WHITE: Self = Self::opaque(255, 255, 255);
    pub const BLACK: Self = Self::opaque(17, 24, 39);
    pub const INK: Self = Self::opaque(31, 41, 55);
    pub const BLUE: Self = Self::opaque(37, 99, 235);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);

    pub fn to_color(self) -> Color {
        Color::from_rgba8(self.r, self.g, self.b, self.a)
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        let (r, g, b) = match s.len() {
            6 => (
                u8::from_str_radix(&s[0..2], 16).ok()?,
                u8::from_str_radix(&s[2..4], 16).ok()?,
                u8::from_str_radix(&s[4..6], 16).ok()?,
            ),
            8 => (
                u8::from_str_radix(&s[0..2], 16).ok()?,
                u8::from_str_radix(&s[2..4], 16).ok()?,
                u8::from_str_radix(&s[4..6], 16).ok()?,
            ),
            _ => return None,
        };
        Some(Self::opaque(r, g, b))
    }

    pub fn from_colorref(v: u32) -> Self {
        Self::opaque((v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8)
    }

    pub fn to_colorref(self) -> u32 {
        (self.r as u32) | ((self.g as u32) << 8) | ((self.b as u32) << 16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorTarget {
    Border,
    Fill,
    Text,
    TextOutline,
}

pub const FONT_CHOICES: &[(&str, &str)] = &[
    ("Microsoft YaHei UI", "微软雅黑"),
    ("SimSun", "宋体"),
    ("SimHei", "黑体"),
    ("KaiTi", "楷体"),
    ("FangSong", "仿宋"),
    ("Segoe UI", "Segoe UI"),
    ("Arial", "Arial"),
    ("Times New Roman", "Times New Roman"),
    ("Consolas", "Consolas"),
];

pub fn font_label(family: &str) -> &str {
    FONT_CHOICES
        .iter()
        .find(|(id, _)| *id == family)
        .map(|(_, label)| *label)
        .unwrap_or(family)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub border: Rgba,
    pub fill: Option<Rgba>,
    pub text: Rgba,
    pub text_outline: Option<Rgba>,
    pub text_outline_width: f32,
    pub font_family: String,
    pub font_size: f32,
    pub stroke_width: f32,
    pub brush_size: f32,
    pub mosaic_size: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            border: Rgba::INK,
            fill: Some(Rgba::WHITE),
            text: Rgba::BLACK,
            text_outline: None,
            text_outline_width: 3.0,
            font_family: "Microsoft YaHei UI".into(),
            font_size: 36.0,
            stroke_width: 3.0,
            brush_size: 10.0,
            mosaic_size: 14.0,
        }
    }
}

pub const PALETTE: [Rgba; 24] = [
    Rgba::opaque(17, 24, 39),
    Rgba::opaque(55, 65, 81),
    Rgba::opaque(107, 114, 128),
    Rgba::opaque(156, 163, 175),
    Rgba::opaque(209, 213, 219),
    Rgba::WHITE,
    Rgba::opaque(239, 68, 68),
    Rgba::opaque(249, 115, 22),
    Rgba::opaque(245, 158, 11),
    Rgba::opaque(250, 204, 21),
    Rgba::opaque(132, 204, 22),
    Rgba::opaque(34, 197, 94),
    Rgba::opaque(16, 185, 129),
    Rgba::opaque(20, 184, 166),
    Rgba::opaque(6, 182, 212),
    Rgba::opaque(14, 165, 233),
    Rgba::opaque(59, 130, 246),
    Rgba::opaque(99, 102, 241),
    Rgba::opaque(139, 92, 246),
    Rgba::opaque(168, 85, 247),
    Rgba::opaque(236, 72, 153),
    Rgba::opaque(244, 114, 182),
    Rgba::opaque(120, 53, 15),
    Rgba::opaque(250, 250, 249),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = Rgba::opaque(18, 52, 86);
        assert_eq!(Rgba::from_hex(&c.to_hex()), Some(c));
    }
}
