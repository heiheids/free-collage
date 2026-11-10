use std::path::Path;

use crate::model::canvas::{CanvasMode, DEFAULT_DPI};
use crate::model::style::{Rgba, Style};

#[derive(Clone, Debug)]
pub struct Settings {
    pub language: String,
    pub canvas: CanvasMode,
    pub style: Style,
    pub export_format: String,
    pub quality: u8,
    pub transparent: bool,
    pub last_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            canvas: CanvasMode::Auto,
            style: Style::default(),
            export_format: "png".into(),
            quality: 90,
            transparent: true,
            last_dir: String::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        let mut fill_none = false;
        let mut fill_color = s.style.fill.unwrap_or(Rgba::WHITE);
        for line in text.trim_start_matches('\u{feff}').lines() {
            let line = line.trim_end_matches('\r').trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            match k.trim() {
                "language" => s.language = v.to_string(),
                "canvas_mode" => {
                    if v.eq_ignore_ascii_case("fixed") {
                        s.canvas = CanvasMode::fixed_a4();
                    } else {
                        s.canvas = CanvasMode::Auto;
                    }
                }
                "canvas_width" => {
                    if let (CanvasMode::Fixed { width_mm, .. }, Ok(x)) = (&mut s.canvas, v.parse()) {
                        *width_mm = x;
                    }
                }
                "canvas_height" => {
                    if let (CanvasMode::Fixed { height_mm, .. }, Ok(x)) = (&mut s.canvas, v.parse()) {
                        *height_mm = x;
                    }
                }
                "canvas_dpi" => {
                    if let (CanvasMode::Fixed { dpi, .. }, Ok(x)) = (&mut s.canvas, v.parse()) {
                        *dpi = x;
                    }
                }
                "border_color" => {
                    if let Some(c) = Rgba::from_hex(v) {
                        s.style.border = c;
                    }
                }
                "fill_none" => fill_none = v == "1" || v.eq_ignore_ascii_case("true"),
                "fill_color" => {
                    if let Some(c) = Rgba::from_hex(v) {
                        fill_color = c;
                    }
                }
                "text_color" => {
                    if let Some(c) = Rgba::from_hex(v) {
                        s.style.text = c;
                    }
                }
                "font_family" => {
                    if !v.is_empty() {
                        s.style.font_family = v.to_string();
                    }
                }
                "text_outline_color" => {
                    s.style.text_outline = if v.eq_ignore_ascii_case("none") {
                        None
                    } else {
                        Rgba::from_hex(v)
                    };
                }
                "text_outline_width" => {
                    if let Ok(x) = v.parse::<f32>() {
                        s.style.text_outline_width = x.clamp(0.0, 80.0);
                    }
                }
                "font_size" => {
                    if let Ok(x) = v.parse::<f32>() {
                        s.style.font_size = x.clamp(6.0, 800.0);
                    }
                }
                "stroke_width" => {
                    if let Ok(x) = v.parse::<f32>() {
                        s.style.stroke_width = x.clamp(0.5, 200.0);
                    }
                }
                "brush_size" => {
                    if let Ok(x) = v.parse::<f32>() {
                        s.style.brush_size = x.clamp(1.0, 300.0);
                    }
                }
                "mosaic_size" => {
                    if let Ok(x) = v.parse::<f32>() {
                        s.style.mosaic_size = x.clamp(3.0, 200.0);
                    }
                }
                "export_format" => {
                    let lower = v.to_ascii_lowercase();
                    if matches!(lower.as_str(), "png" | "jpeg" | "webp" | "bmp") {
                        s.export_format = lower;
                    }
                }
                "quality" => {
                    if let Ok(x) = v.parse::<u8>() {
                        s.quality = x.clamp(1, 100);
                    }
                }
                "transparent" => s.transparent = v == "1" || v.eq_ignore_ascii_case("true"),
                "last_dir" => s.last_dir = v.to_string(),
                _ => {}
            }
        }
        if let CanvasMode::Fixed {
            width_mm,
            height_mm,
            dpi,
        } = s.canvas
        {
            s.canvas = CanvasMode::clamp_fixed(width_mm, height_mm, dpi);
        }
        s.style.fill = if fill_none { None } else { Some(fill_color) };
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut text = String::new();
        text.push_str("# 自由拼图 settings\n");
        text.push_str("# 修改后重启程序生效\n");
        text.push_str(&format!("language={}\n", self.language));
        match &self.canvas {
            CanvasMode::Auto => {
                text.push_str("canvas_mode=auto\n");
                text.push_str(&format!("canvas_width={}\n", 210.0));
                text.push_str(&format!("canvas_height={}\n", 297.0));
                text.push_str(&format!("canvas_dpi={}\n", DEFAULT_DPI));
            }
            CanvasMode::Fixed {
                width_mm,
                height_mm,
                dpi,
            } => {
                text.push_str("canvas_mode=fixed\n");
                text.push_str(&format!("canvas_width={width_mm}\n"));
                text.push_str(&format!("canvas_height={height_mm}\n"));
                text.push_str(&format!("canvas_dpi={dpi}\n"));
            }
        }
        text.push_str(&format!("border_color={}\n", self.style.border.to_hex()));
        text.push_str(&format!(
            "fill_none={}\n",
            if self.style.fill.is_some() { 0 } else { 1 }
        ));
        text.push_str(&format!(
            "fill_color={}\n",
            self.style.fill.unwrap_or(Rgba::WHITE).to_hex()
        ));
        text.push_str(&format!("text_color={}\n", self.style.text.to_hex()));
        text.push_str(&format!("font_family={}\n", self.style.font_family));
        text.push_str(&format!(
            "text_outline_color={}\n",
            self.style
                .text_outline
                .map(|c| c.to_hex())
                .unwrap_or_else(|| "none".into())
        ));
        text.push_str(&format!(
            "text_outline_width={}\n",
            self.style.text_outline_width
        ));
        text.push_str(&format!("font_size={}\n", self.style.font_size));
        text.push_str(&format!("stroke_width={}\n", self.style.stroke_width));
        text.push_str(&format!("brush_size={}\n", self.style.brush_size));
        text.push_str(&format!("mosaic_size={}\n", self.style.mosaic_size));
        text.push_str(&format!("export_format={}\n", self.export_format));
        text.push_str(&format!("quality={}\n", self.quality));
        text.push_str(&format!("transparent={}\n", if self.transparent { 1 } else { 0 }));
        text.push_str(&format!("last_dir={}\n", self.last_dir));
        std::fs::write(path, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip() {
        let mut s = Settings::default();
        s.language = "zh-CN".into();
        s.canvas = CanvasMode::clamp_fixed(100.0, 200.0, 150.0);
        s.style.border = Rgba::opaque(1, 2, 3);
        s.style.fill = None;
        s.quality = 42;
        let text = format!(
            "language=zh-CN\ncanvas_mode=fixed\ncanvas_width=100\ncanvas_height=200\ncanvas_dpi=150\nborder_color=#010203\nfill_none=1\nquality=42\n"
        );
        let parsed = Settings::parse(&text);
        assert_eq!(parsed.language, "zh-CN");
        assert_eq!(parsed.canvas.pixel_size(), s.canvas.pixel_size());
        assert_eq!(parsed.style.border, Rgba::opaque(1, 2, 3));
        assert!(parsed.style.fill.is_none());
        assert_eq!(parsed.quality, 42);
    }

    #[test]
    fn font_and_outline_settings_parse() {
        let parsed = Settings::parse(
            "font_family=SimSun\ntext_outline_color=#FF0000\ntext_outline_width=5\n",
        );
        assert_eq!(parsed.style.font_family, "SimSun");
        assert_eq!(parsed.style.text_outline, Some(Rgba::opaque(255, 0, 0)));
        assert_eq!(parsed.style.text_outline_width, 5.0);
        let none = Settings::parse("text_outline_color=none\n");
        assert!(none.style.text_outline.is_none());
    }
}
