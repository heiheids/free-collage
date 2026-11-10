use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, OnceLock};

use ab_glyph::{point, Font, FontVec, Glyph, PxScale, ScaleFont};
use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::{COLORREF, HWND, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW,
    GetDC, GetStockObject, GetTextExtentPoint32W, GetTextMetricsW, ReleaseDC, SelectObject,
    SetBkMode, SetTextAlign, SetTextColor, StretchDIBits, ANTIALIASED_QUALITY, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    DIB_RGB_COLORS, DT_LEFT, DT_NOCLIP, DT_NOPREFIX, DT_TOP, FF_DONTCARE, HDC, HFONT,
    HGDIOBJ, SRCCOPY, TA_LEFT, TA_TOP, TEXTMETRICW, TRANSPARENT, WHITE_BRUSH,
};

use crate::model::style::Rgba;

pub const TEXT_SCALE: f32 = 1.14;
pub const DEFAULT_FONT: &str = "Microsoft YaHei UI";

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct MaskKey {
    text: String,
    size: i32,
    weight: u32,
    family: String,
}

pub struct TextMask {
    pub width: i32,
    pub height: i32,
    pub alpha: Vec<u8>,
}

pub struct TextEngine {
    fonts: HashMap<(i32, u32, String), HFONT>,
    masks: HashMap<MaskKey, Arc<TextMask>>,
    metrics: HashMap<MaskKey, (f32, f32)>,
    measure_dc: HDC,
    system_fonts: Option<Arc<SystemFonts>>,
    families: HashMap<String, Arc<SystemFonts>>,
}

static SYSTEM_FONTS: OnceLock<Option<Arc<SystemFonts>>> = OnceLock::new();

struct SystemFonts {
    regular: FontVec,
    bold: Option<FontVec>,
}

impl SystemFonts {
    fn face(&self, weight: u32) -> &FontVec {
        if weight >= 600 {
            self.bold.as_ref().unwrap_or(&self.regular)
        } else {
            &self.regular
        }
    }
}

impl Drop for TextEngine {
    fn drop(&mut self) {
        unsafe {
            if !self.measure_dc.is_null() {
                DeleteDC(self.measure_dc);
            }
            for font in self.fonts.values() {
                if !font.is_null() {
                    DeleteObject(*font as HGDIOBJ);
                }
            }
        }
    }
}

impl TextEngine {
    pub fn new() -> Self {
        let measure_dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        Self {
            fonts: HashMap::new(),
            masks: HashMap::new(),
            metrics: HashMap::new(),
            measure_dc,
            system_fonts: load_system_fonts_cached(),
            families: HashMap::new(),
        }
    }

    pub fn font(&mut self, size: f32, weight: u32) -> HFONT {
        self.font_family(size, weight, DEFAULT_FONT)
    }

    pub fn font_family(&mut self, size: f32, weight: u32, family: &str) -> HFONT {
        let px = size.round().clamp(1.0, 800.0) as i32;
        let family = if family.is_empty() { DEFAULT_FONT } else { family };
        let key = (px, weight, family.to_string());
        if let Some(h) = self.fonts.get(&key) {
            return *h;
        }
        let face: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
        let font = unsafe {
            CreateFontW(
                -px,
                0,
                0,
                0,
                weight as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                0,
                CLIP_DEFAULT_PRECIS as u32,
                ANTIALIASED_QUALITY as u32,
                (DEFAULT_PITCH | FF_DONTCARE) as u32,
                face.as_ptr(),
            )
        };
        self.fonts.insert(key, font);
        font
    }

    pub fn measure(&mut self, text: &str, size: f32, weight: u32) -> (f32, f32) {
        self.measure_family(text, size, weight, DEFAULT_FONT)
    }

    pub fn measure_family(
        &mut self,
        text: &str,
        size: f32,
        weight: u32,
        family: &str,
    ) -> (f32, f32) {
        let px = size.round().clamp(1.0, 800.0) as i32;
        let key = MaskKey {
            text: text.to_string(),
            size: px,
            weight,
            family: family.to_string(),
        };
        if let Some(m) = self.metrics.get(&key) {
            return *m;
        }
        let Some(fonts) = self.fonts_for(family) else {
            return self.measure_gdi(text, size, weight);
        };
        let m = ab_measure(&fonts, text, size, weight);
        self.metrics.insert(key, m);
        m
    }

    fn measure_gdi(&mut self, text: &str, size: f32, weight: u32) -> (f32, f32) {
        let px = size.round().clamp(1.0, 800.0) as i32;
        let key = MaskKey {
            text: text.to_string(),
            size: px,
            weight,
            family: DEFAULT_FONT.to_string(),
        };
        if let Some(m) = self.metrics.get(&key) {
            return *m;
        }
        let hdc = self.measure_dc;
        if hdc.is_null() {
            return (size, size);
        }
        let font = self.font(size, weight);
        let old = unsafe { SelectObject(hdc, font as HGDIOBJ) };
        let mut line_height = px + 4;
        unsafe {
            let mut tm: TEXTMETRICW = std::mem::zeroed();
            if GetTextMetricsW(hdc, &mut tm) != 0 {
                line_height = tm.tmHeight + tm.tmExternalLeading;
            }
        }
        let lines: Vec<&str> = text.split('\n').collect();
        let mut width = 0i32;
        for line in &lines {
            let line = line.trim_end_matches('\r');
            let wide: Vec<u16> = line.encode_utf16().collect();
            let mut sz = SIZE { cx: 0, cy: 0 };
            unsafe {
                GetTextExtentPoint32W(hdc, wide.as_ptr(), wide.len() as i32, &mut sz);
            }
            width = width.max(sz.cx);
        }
        let w = (width + 4).max(2) as f32;
        let h = (line_height.max(1) * lines.len().max(1) as i32 + 4) as f32;
        unsafe {
            SelectObject(hdc, old);
        }
        self.metrics.insert(key, (w, h));
        (w, h)
    }

    pub fn mask(&mut self, text: &str, size: f32, weight: u32) -> Option<Arc<TextMask>> {
        self.mask_family(text, size, weight, DEFAULT_FONT)
    }

    pub fn mask_family(
        &mut self,
        text: &str,
        size: f32,
        weight: u32,
        family: &str,
    ) -> Option<Arc<TextMask>> {
        let px = size.round().clamp(1.0, 800.0) as i32;
        let key = MaskKey {
            text: text.to_string(),
            size: px,
            weight,
            family: family.to_string(),
        };
        if let Some(m) = self.masks.get(&key) {
            return Some(m.clone());
        }
        if self.masks.len() > 600 {
            self.masks.clear();
        }
        let fonts = self.fonts_for(family)?;
        let (fw, fh) = self.measure_family(text, size, weight, family);
        let width = fw.ceil().clamp(1.0, 8192.0) as i32;
        let height = fh.ceil().clamp(1.0, 8192.0) as i32;
        let alpha = ab_mask(&fonts, text, size, weight, width, height);
        let mask = Arc::new(TextMask {
            width,
            height,
            alpha,
        });
        self.masks.insert(key, mask.clone());
        Some(mask)
    }

    fn mask_gdi(&mut self, text: &str, size: f32, weight: u32) -> Option<Arc<TextMask>> {
        let px = size.round().clamp(1.0, 800.0) as i32;
        let key = MaskKey {
            text: text.to_string(),
            size: px,
            weight,
            family: DEFAULT_FONT.to_string(),
        };
        if let Some(m) = self.masks.get(&key) {
            return Some(m.clone());
        }
        if self.masks.len() > 600 {
            self.masks.clear();
        }
        let (fw, fh) = self.measure_gdi(text, size, weight);
        let width = fw.ceil().clamp(1.0, 8192.0) as i32;
        let height = fh.ceil().clamp(1.0, 8192.0) as i32;
        let hdc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if hdc.is_null() {
            return None;
        }
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: (width * height * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows_sys::Win32::Graphics::Gdi::RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = unsafe {
            CreateDIBSection(
                hdc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            )
        };
        if bitmap.is_null() || bits.is_null() {
            unsafe {
                DeleteDC(hdc);
            }
            return None;
        }
        let old_bmp = unsafe { SelectObject(hdc, bitmap as HGDIOBJ) };
        unsafe {
            std::ptr::write_bytes(bits, 0, (width * height * 4) as usize);
        }
        let font = self.font(size, weight);
        let old_font = unsafe { SelectObject(hdc, font as HGDIOBJ) };
        let wide: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, 0x00FF_FFFF as COLORREF);
            SetTextAlign(hdc, TA_LEFT | TA_TOP);
            let mut rc = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            DrawTextW(
                hdc,
                wide.as_ptr(),
                wide.len() as i32,
                &mut rc,
                DT_LEFT | DT_TOP | DT_NOPREFIX | DT_NOCLIP,
            );
            windows_sys::Win32::Graphics::Gdi::GdiFlush();
        }
        let mut alpha = vec![0u8; (width * height) as usize];
        unsafe {
            let src = bits as *const u8;
            for i in 0..alpha.len() {
                let b = *src.add(i * 4);
                let g = *src.add(i * 4 + 1);
                let r = *src.add(i * 4 + 2);
                alpha[i] = r.max(g).max(b);
            }
            SelectObject(hdc, old_font);
            SelectObject(hdc, old_bmp);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(hdc);
        }
        let mask = Arc::new(TextMask {
            width,
            height,
            alpha,
        });
        self.masks.insert(key, mask.clone());
        Some(mask)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        pixmap: &mut Pixmap,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: u32,
        color: Rgba,
        opacity: f32,
    ) {
        self.draw_family(
            pixmap,
            x,
            y,
            text,
            size,
            weight,
            color,
            opacity,
            DEFAULT_FONT,
            None,
            0.0,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_family(
        &mut self,
        pixmap: &mut Pixmap,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: u32,
        color: Rgba,
        opacity: f32,
        family: &str,
        outline: Option<Rgba>,
        outline_width: f32,
    ) {
        if text.is_empty() {
            return;
        }
        let Some(mask) = self.mask_family(text, size, weight, family) else {
            return;
        };
        let opacity = (opacity.clamp(0.0, 1.0) * 255.0) as u32;
        let px = x.round() as i32;
        let py = y.round() as i32;
        if let Some(outline) = outline {
            if outline.a > 0 && outline_width > 0.5 {
                let radius = outline_width.max(1.0);
                for step in 0..16 {
                    let angle = step as f32 * std::f32::consts::TAU / 16.0;
                    let dx = (angle.cos() * radius).round() as i32;
                    let dy = (angle.sin() * radius).round() as i32;
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    blit_mask(pixmap, &mask, px + dx, py + dy, outline, opacity);
                }
            }
        }
        blit_mask(pixmap, &mask, px, py, color, opacity);
    }

    fn fonts_for(&mut self, family: &str) -> Option<Arc<SystemFonts>> {
        let family = if family.is_empty() {
            DEFAULT_FONT
        } else {
            family
        };
        if family == DEFAULT_FONT {
            if let Some(default) = &self.system_fonts {
                return Some(default.clone());
            }
        }
        if let Some(fonts) = self.families.get(family) {
            return Some(fonts.clone());
        }
        if let Some(fonts) = load_system_fonts_named(family) {
            let fonts = Arc::new(fonts);
            self.families.insert(family.to_string(), fonts.clone());
            return Some(fonts);
        }
        self.system_fonts.clone()
    }

    pub fn clear_masks(&mut self) {
        self.masks.clear();
        self.metrics.clear();
    }
}

fn blit_mask(pixmap: &mut Pixmap, mask: &TextMask, px: i32, py: i32, color: Rgba, opacity: u32) {
    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;
    let data = pixmap.data_mut();
    for row in 0..mask.height {
        let ty = py + row;
        if ty < 0 || ty >= ph {
            continue;
        }
        for col in 0..mask.width {
            let tx = px + col;
            if tx < 0 || tx >= pw {
                continue;
            }
            let a = mask.alpha[(row * mask.width + col) as usize] as u32;
            let sa = a * opacity / 255;
            if sa == 0 {
                continue;
            }
            let idx = ((ty * pw + tx) * 4) as usize;
            let inv = 255 - sa;
            let sr = color.r as u32 * sa / 255;
            let sg = color.g as u32 * sa / 255;
            let sb = color.b as u32 * sa / 255;
            data[idx] = (sr + data[idx] as u32 * inv / 255).min(255) as u8;
            data[idx + 1] = (sg + data[idx + 1] as u32 * inv / 255).min(255) as u8;
            data[idx + 2] = (sb + data[idx + 2] as u32 * inv / 255).min(255) as u8;
            data[idx + 3] = (sa + data[idx + 3] as u32 * inv / 255).min(255) as u8;
        }
    }
}

fn load_system_fonts_cached() -> Option<Arc<SystemFonts>> {
    SYSTEM_FONTS
        .get_or_init(|| load_system_fonts().map(Arc::new))
        .clone()
}

fn load_system_fonts() -> Option<SystemFonts> {
    if let Some(fonts) = load_system_fonts_named(DEFAULT_FONT) {
        return Some(fonts);
    }
    let dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join("Fonts");
    let regular_path = ["msyh.ttc", "msyhbd.ttc", "simhei.ttf", "segoeui.ttf"]
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())?;
    let regular = load_face(&regular_path)?;
    let bold_path = ["msyhbd.ttc", "msyh.ttc"]
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file());
    let bold = bold_path
        .filter(|path| *path != regular_path)
        .and_then(|path| load_face(&path));
    Some(SystemFonts { regular, bold })
}

struct FontSpec {
    file: &'static str,
    index: u32,
    bold_file: &'static str,
    bold_index: u32,
}

fn font_spec(family: &str) -> Option<FontSpec> {
    let spec = match family {
        "Microsoft YaHei UI" => ("msyh.ttc", 1, "msyhbd.ttc", 1),
        "SimSun" => ("simsun.ttc", 0, "simsun.ttc", 0),
        "SimHei" => ("simhei.ttf", 0, "simhei.ttf", 0),
        "KaiTi" => ("simkai.ttf", 0, "simkai.ttf", 0),
        "FangSong" => ("simfang.ttf", 0, "simfang.ttf", 0),
        "Segoe UI" => ("segoeui.ttf", 0, "segoeuib.ttf", 0),
        "Arial" => ("arial.ttf", 0, "arialbd.ttf", 0),
        "Times New Roman" => ("times.ttf", 0, "timesbd.ttf", 0),
        "Consolas" => ("consola.ttf", 0, "consolab.ttf", 0),
        _ => return None,
    };
    Some(FontSpec {
        file: spec.0,
        index: spec.1,
        bold_file: spec.2,
        bold_index: spec.3,
    })
}

fn load_system_fonts_named(family: &str) -> Option<SystemFonts> {
    let spec = font_spec(family)?;
    let dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join("Fonts");
    let regular_path = dir.join(spec.file);
    let regular = load_face_index(&regular_path, spec.index)
        .or_else(|| load_face(&regular_path))?;
    let bold_path = dir.join(spec.bold_file);
    let bold = if bold_path == regular_path {
        None
    } else {
        load_face_index(&bold_path, spec.bold_index).or_else(|| load_face(&bold_path))
    };
    Some(SystemFonts { regular, bold })
}

fn load_face_index(path: &std::path::Path, index: u32) -> Option<FontVec> {
    let data = std::fs::read(path).ok()?;
    FontVec::try_from_vec_and_index(data, index).ok()
}

fn load_face(path: &std::path::Path) -> Option<FontVec> {
    let data = std::fs::read(path).ok()?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let indices: &[u32] = if name.starts_with("msyh") {
        // Microsoft YaHei TTC face 1 is the UI variant used by Windows.
        &[1, 0, 2, 3]
    } else {
        &[0, 1, 2, 3]
    };
    for &index in indices {
        if let Ok(font) = FontVec::try_from_vec_and_index(data.clone(), index) {
            return Some(font);
        }
    }
    None
}

fn ab_measure(fonts: &SystemFonts, text: &str, size: f32, weight: u32) -> (f32, f32) {
    let face = fonts.face(weight);
    let size = (size * TEXT_SCALE).max(1.0);
    let scale = PxScale::from(size);
    let scaled = face.as_scaled(scale);
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let line_gap = scaled.line_gap().max(0.0);
    let line_height = (ascent - descent + line_gap).max(size * 1.05);
    let lines: Vec<&str> = text.split('\n').collect();
    let space = scaled.h_advance(scaled.glyph_id(' '));
    let mut max_width = 0.0f32;
    for line in &lines {
        let line = line.trim_end_matches('\r');
        let mut width = 0.0f32;
        let mut prev = None;
        for ch in line.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(p) = prev {
                width += scaled.kern(p, id);
            }
            if ch == '\t' {
                width += space * 4.0;
                prev = None;
            } else {
                width += scaled.h_advance(id);
                prev = Some(id);
            }
        }
        max_width = max_width.max(width);
    }
    let w = (max_width.ceil() + 6.0).max(2.0);
    let h = (line_height * lines.len().max(1) as f32 + 6.0).ceil();
    (w, h)
}

fn ab_mask(
    fonts: &SystemFonts,
    text: &str,
    size: f32,
    weight: u32,
    width: i32,
    height: i32,
) -> Vec<u8> {
    let mut alpha = vec![0u8; (width * height) as usize];
    if width <= 0 || height <= 0 {
        return alpha;
    }
    let face = fonts.face(weight);
    let size = (size * TEXT_SCALE).max(1.0);
    let scale = PxScale::from(size);
    let scaled = face.as_scaled(scale);
    let ascent = scaled.ascent();
    let descent = scaled.descent();
    let line_gap = scaled.line_gap().max(0.0);
    let line_height = (ascent - descent + line_gap).max(size * 1.05);
    let space = scaled.h_advance(scaled.glyph_id(' '));
    for (line_index, line) in text.split('\n').enumerate() {
        let line = line.trim_end_matches('\r');
        let baseline = 3.0 + ascent + line_index as f32 * line_height;
        let mut pen = 3.0f32;
        let mut prev = None;
        for ch in line.chars() {
            let id = scaled.glyph_id(ch);
            if let Some(p) = prev {
                pen += scaled.kern(p, id);
            }
            if ch == '\t' {
                pen += space * 4.0;
                prev = None;
                continue;
            }
            if ch != ' ' {
                let glyph = Glyph {
                    id,
                    scale,
                    position: point(pen, baseline),
                };
                if let Some(outline) = scaled.outline_glyph(glyph) {
                    let bounds = outline.px_bounds();
                    let ox = bounds.min.x as i32;
                    let oy = bounds.min.y as i32;
                    outline.draw(|x, y, coverage| {
                        if coverage <= 0.0 {
                            return;
                        }
                        let tx = ox + x as i32;
                        let ty = oy + y as i32;
                        if tx < 0 || ty < 0 || tx >= width || ty >= height {
                            return;
                        }
                        let value = (coverage.clamp(0.0, 1.0).powf(0.78) * 255.0).round() as u8;
                        let idx = (ty * width + tx) as usize;
                        if value > alpha[idx] {
                            alpha[idx] = value;
                        }
                    });
                }
            }
            pen += scaled.h_advance(id);
            prev = Some(id);
        }
    }
    alpha
}

pub fn white_brush() -> *mut c_void {
    unsafe { GetStockObject(WHITE_BRUSH) as *mut c_void }
}

pub fn present(hdc: HDC, width: i32, height: i32, pixmap: &Pixmap, buffer: &mut Vec<u32>) {
    if width <= 0 || height <= 0 {
        return;
    }
    let n = (width * height) as usize;
    if buffer.len() != n {
        buffer.resize(n, 0);
    }
    let src = pixmap.data();
    for (s, d) in src.chunks_exact(4).zip(buffer.iter_mut()) {
        *d = 0xFF00_0000
            | ((s[0] as u32) << 16)
            | ((s[1] as u32) << 8)
            | s[2] as u32;
    }
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: (width * height * 4) as u32,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [windows_sys::Win32::Graphics::Gdi::RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };
    unsafe {
        StretchDIBits(
            hdc,
            0,
            0,
            width,
            height,
            0,
            0,
            width,
            height,
            buffer.as_ptr() as *const c_void,
            &bmi,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

pub fn screen_dc(hwnd: HWND) -> HDC {
    unsafe { GetDC(hwnd) }
}

pub fn release_dc(hwnd: HWND, hdc: HDC) {
    unsafe {
        ReleaseDC(hwnd, hdc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn antialiased_cjk_mask_has_ink() {
        let mut engine = TextEngine::new();
        assert!(engine.system_fonts.is_some(), "system CJK font must load");
        let (w, h) = engine.measure("拼图测试 ABC", 32.0, 600);
        assert!(w > 80.0 && h > 20.0, "unexpected text metrics: {w}x{h}");
        let mask = engine.mask("拼图测试 ABC", 32.0, 600).unwrap();
        let ink = mask.alpha.iter().filter(|value| **value > 0).count();
        assert!(ink > 200, "text mask is almost empty: {ink}");
        assert_eq!((mask.width, mask.height), (w.ceil() as i32, h.ceil() as i32));
    }

    #[test]
    fn font_family_and_outline_render() {
        let mut engine = TextEngine::new();
        let (w, h) = engine.measure_family("H", 42.0, 700, "Segoe UI");
        assert!(w > 10.0 && h > 20.0);
        let mut pixmap = Pixmap::new(180, 90).unwrap();
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
        engine.draw_family(
            &mut pixmap,
            8.0,
            8.0,
            "H",
            42.0,
            700,
            Rgba::WHITE,
            1.0,
            "Segoe UI",
            Some(Rgba::opaque(255, 0, 0)),
            4.0,
        );
        let red = pixmap
            .data()
            .chunks_exact(4)
            .filter(|p| p[0] > 140 && p[1] < 90 && p[2] < 90 && p[3] > 0)
            .count();
        assert!(red > 20, "outline did not draw: {red} red pixels");
    }
}
