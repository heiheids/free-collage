use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use image::codecs::bmp::BmpEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};
use tiny_skia::Pixmap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Webp,
    Bmp,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "jpeg" | "jpg" => Self::Jpeg,
            "webp" => Self::Webp,
            "bmp" => Self::Bmp,
            _ => Self::Png,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            "bmp" => Some(Self::Bmp),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
            Self::Bmp => "bmp",
        }
    }

    pub fn supports_alpha(self) -> bool {
        matches!(self, Self::Png | Self::Webp)
    }

    pub fn quality_enabled(self) -> bool {
        matches!(self, Self::Jpeg | Self::Webp)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Webp => "WebP",
            Self::Bmp => "BMP",
        }
    }

    pub const ALL: [Self; 4] = [Self::Png, Self::Jpeg, Self::Webp, Self::Bmp];
}

pub fn load_pixmap(path: &Path) -> Result<Pixmap, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    decode_bytes(&bytes)
}

pub fn decode_bytes(bytes: &[u8]) -> Result<Pixmap, String> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let image = webp::Decoder::new(bytes)
            .decode()
            .ok_or_else(|| "WebP decode failed (animated WebP is not supported)".to_string())?;
        let w = image.width();
        let h = image.height();
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        if image.is_alpha() {
            rgba.extend_from_slice(&image);
        } else {
            for chunk in image.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        return pixmap_from_rgba(w, h, rgba);
    }
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let mut rgba = rgba.into_raw();
    ensure_alpha(&mut rgba);
    pixmap_from_rgba(img.width(), img.height(), rgba)
}

fn ensure_alpha(rgba: &mut [u8]) {
    let has_alpha = rgba.chunks_exact(4).any(|p| p[3] != 0);
    if !has_alpha {
        for p in rgba.chunks_exact_mut(4) {
            p[3] = 255;
        }
    }
}

pub fn pixmap_from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Pixmap, String> {
    if width == 0 || height == 0 {
        return Err("empty image".into());
    }
    if width as u64 * height as u64 > 150_000_000 {
        return Err("image is too large".into());
    }
    if rgba.len() != (width * height * 4) as usize {
        return Err("invalid image buffer".into());
    }
    if width > 20000 || height > 20000 {
        return Err("image dimensions are too large".into());
    }
    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| "cannot allocate image".to_string())?;
    let dst = pixmap.data_mut();
    for (src, out) in rgba.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        let a = src[3] as u32;
        out[0] = ((src[0] as u32 * a + 127) / 255) as u8;
        out[1] = ((src[1] as u32 * a + 127) / 255) as u8;
        out[2] = ((src[2] as u32 * a + 127) / 255) as u8;
        out[3] = src[3];
    }
    Ok(pixmap)
}

pub fn pixmap_to_rgba(pixmap: &Pixmap) -> Vec<u8> {
    let mut out = vec![0u8; pixmap.data().len()];
    for (src, dst) in pixmap.data().chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        let a = src[3] as u32;
        if a == 0 {
            dst[0] = 0;
            dst[1] = 0;
            dst[2] = 0;
            dst[3] = 0;
        } else {
            dst[0] = ((src[0] as u32 * 255 + a / 2) / a).min(255) as u8;
            dst[1] = ((src[1] as u32 * 255 + a / 2) / a).min(255) as u8;
            dst[2] = ((src[2] as u32 * 255 + a / 2) / a).min(255) as u8;
            dst[3] = a as u8;
        }
    }
    out
}

pub fn write_image(
    path: &Path,
    format: ExportFormat,
    pixmap: &Pixmap,
    quality: u8,
) -> Result<u64, String> {
    let rgba = pixmap_to_rgba(pixmap);
    write_rgba(path, format, &rgba, pixmap.width(), pixmap.height(), quality)
}

pub fn write_rgba(
    path: &Path,
    format: ExportFormat,
    rgba: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<u64, String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let quality = quality.clamp(1, 100);
    match format {
        ExportFormat::Png => {
            let file = File::create(path).map_err(|e| e.to_string())?;
            let writer = BufWriter::new(file);
            let encoder =
                PngEncoder::new_with_quality(writer, CompressionType::Default, FilterType::Adaptive);
            encoder
                .write_image(rgba, width, height, ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?;
        }
        ExportFormat::Jpeg => {
            let file = File::create(path).map_err(|e| e.to_string())?;
            let writer = BufWriter::new(file);
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for p in rgba.chunks_exact(4) {
                rgb.push(p[0]);
                rgb.push(p[1]);
                rgb.push(p[2]);
            }
            let encoder = JpegEncoder::new_with_quality(writer, quality);
            encoder
                .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
                .map_err(|e| e.to_string())?;
        }
        ExportFormat::Webp => {
            let encoded = webp::Encoder::from_rgba(rgba, width, height).encode(quality as f32);
            let mut file = File::create(path).map_err(|e| e.to_string())?;
            file.write_all(&encoded).map_err(|e| e.to_string())?;
        }
        ExportFormat::Bmp => {
            let file = File::create(path).map_err(|e| e.to_string())?;
            let mut writer = BufWriter::new(file);
            let encoder = BmpEncoder::new(&mut writer);
            encoder
                .write_image(rgba, width, height, ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))
}

pub fn rgba_image_to_pixmap(img: RgbaImage) -> Result<Pixmap, String> {
    let (w, h) = img.dimensions();
    pixmap_from_rgba(w, h, img.into_raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::style::Rgba;

    #[test]
    fn premultiply_roundtrip() {
        let mut p = Pixmap::new(2, 2).unwrap();
        p.fill(Rgba::new(120, 60, 30, 128).to_color());
        let rgba = pixmap_to_rgba(&p);
        assert_eq!(rgba[0], 120);
        assert_eq!(rgba[3], 128);
    }

    #[test]
    fn format_detection() {
        assert_eq!(ExportFormat::from_path(Path::new("a.webp")), Some(ExportFormat::Webp));
        assert_eq!(ExportFormat::from_path(Path::new("a.jpeg")), Some(ExportFormat::Jpeg));
        assert_eq!(ExportFormat::from_path(Path::new("a.unknown")), None);
    }
}
