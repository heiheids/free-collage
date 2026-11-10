#[derive(Clone, Debug, PartialEq)]
pub enum CanvasMode {
    Auto,
    Fixed {
        width_mm: f32,
        height_mm: f32,
        dpi: f32,
    },
}

pub const A4_MM: (f32, f32) = (210.0, 297.0);
pub const B5_MM: (f32, f32) = (176.0, 250.0);
pub const DEFAULT_DPI: f32 = 300.0;
pub const MIN_MM: f32 = 10.0;
pub const MAX_MM: f32 = 2000.0;
pub const MIN_DPI: f32 = 36.0;
pub const MAX_DPI: f32 = 1200.0;

impl Default for CanvasMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl CanvasMode {
    pub fn fixed_a4() -> Self {
        Self::Fixed {
            width_mm: A4_MM.0,
            height_mm: A4_MM.1,
            dpi: DEFAULT_DPI,
        }
    }

    pub fn fixed_b5() -> Self {
        Self::Fixed {
            width_mm: B5_MM.0,
            height_mm: B5_MM.1,
            dpi: DEFAULT_DPI,
        }
    }

    pub fn fixed_a4_landscape() -> Self {
        Self::Fixed {
            width_mm: A4_MM.1,
            height_mm: A4_MM.0,
            dpi: DEFAULT_DPI,
        }
    }

    pub fn fixed_b5_landscape() -> Self {
        Self::Fixed {
            width_mm: B5_MM.1,
            height_mm: B5_MM.0,
            dpi: DEFAULT_DPI,
        }
    }

    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    pub fn pixel_size(&self) -> Option<(u32, u32)> {
        match self {
            Self::Auto => None,
            Self::Fixed {
                width_mm,
                height_mm,
                dpi,
            } => {
                let w = (width_mm * dpi / 25.4).round().clamp(1.0, 20000.0) as u32;
                let h = (height_mm * dpi / 25.4).round().clamp(1.0, 20000.0) as u32;
                Some((w, h))
            }
        }
    }

    pub fn clamp_fixed(width_mm: f32, height_mm: f32, dpi: f32) -> Self {
        Self::Fixed {
            width_mm: width_mm.clamp(MIN_MM, MAX_MM),
            height_mm: height_mm.clamp(MIN_MM, MAX_MM),
            dpi: dpi.clamp(MIN_DPI, MAX_DPI),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_pixels_at_300dpi() {
        assert_eq!(CanvasMode::fixed_a4().pixel_size(), Some((2480, 3508)));
    }

    #[test]
    fn b5_pixels_at_300dpi() {
        assert_eq!(CanvasMode::fixed_b5().pixel_size(), Some((2079, 2953)));
    }

    #[test]
    fn landscape_pixels_at_300dpi() {
        assert_eq!(CanvasMode::fixed_a4_landscape().pixel_size(), Some((3508, 2480)));
        assert_eq!(CanvasMode::fixed_b5_landscape().pixel_size(), Some((2953, 2079)));
    }
}
