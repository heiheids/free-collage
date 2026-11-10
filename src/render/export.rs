use tiny_skia::Pixmap;

use crate::codec::ExportFormat;
use crate::model::{Document, Point, Rect};
use crate::platform::gdi::TextEngine;
use crate::render::doc::{crop_transparent, render_document, Background};

pub struct ExportImage {
    pub pixmap: Pixmap,
    pub page: Rect,
}

pub fn render_for_export(
    doc: &Document,
    format: ExportFormat,
    transparent: bool,
    text: &mut TextEngine,
) -> Result<ExportImage, String> {
    let page = doc.page_rect().ok_or_else(|| "no content".to_string())?;
    let (w, h) = if let Some((w, h)) = doc.canvas.pixel_size() {
        (w, h)
    } else {
        (page.w.ceil().max(1.0) as u32, page.h.ceil().max(1.0) as u32)
    };
    if w > 20000 || h > 20000 {
        return Err("output is too large".into());
    }
    let alpha = transparent && format.supports_alpha();
    let background = if alpha {
        Background::Transparent
    } else {
        Background::White
    };
    let pixmap = render_document(
        doc,
        page,
        1.0,
        Point::new(-page.x, -page.y),
        w,
        h,
        background,
        text,
        None,
        8.0,
        true,
    )
    .ok_or_else(|| "cannot render output".to_string())?;

    if doc.canvas.is_auto() {
        if alpha {
            let cropped = crop_transparent(pixmap).ok_or_else(|| "no visible content".to_string())?;
            return Ok(ExportImage {
                pixmap: cropped,
                page,
            });
        }
    }
    Ok(ExportImage { pixmap, page })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{self, ExportFormat};
    use crate::model::canvas::CanvasMode;
    use crate::model::layer::{Layer, ShapeKind};
    use crate::model::{Document, Point, Rect, Rgba};
    use crate::platform::gdi::TextEngine;
    use std::path::PathBuf;

    fn test_dir() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-output");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fixed_doc() -> Document {
        let mut doc = Document::new();
        doc.canvas = CanvasMode::Fixed {
            width_mm: 200.0 * 25.4 / 300.0,
            height_mm: 100.0 * 25.4 / 300.0,
            dpi: 300.0,
        };
        doc.add(Layer::shape(
            0,
            ShapeKind::Rect,
            Point::new(40.0, 20.0),
            Point::new(160.0, 80.0),
            Rgba::TRANSPARENT,
            Some(Rgba::opaque(220, 40, 40)),
            0.5,
            0.0,
        ));
        doc
    }

    fn px(pixmap: &tiny_skia::Pixmap, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let p = pixmap.pixel(x, y).unwrap();
        (p.red(), p.green(), p.blue(), p.alpha())
    }

    #[test]
    fn png_and_webp_keep_transparency() {
        let doc = fixed_doc();
        let mut text = TextEngine::new();
        for format in [ExportFormat::Png, ExportFormat::Webp] {
            let img = render_for_export(&doc, format, true, &mut text).unwrap();
            assert_eq!((img.pixmap.width(), img.pixmap.height()), (200, 100));
            assert_eq!(px(&img.pixmap, 0, 0).3, 0);
            assert_eq!(px(&img.pixmap, 100, 50).3, 255);
            let path = test_dir().join(format!("alpha.{}", format.ext()));
            let bytes = codec::write_image(&path, format, &img.pixmap, 80).unwrap();
            assert!(bytes > 100);
            let decoded = codec::load_pixmap(&path).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (200, 100));
            assert_eq!(px(&decoded, 0, 0).3, 0);
            assert_eq!(px(&decoded, 100, 50).3, 255);
        }
    }

    #[test]
    fn jpeg_is_filled_with_white() {
        let doc = fixed_doc();
        let mut text = TextEngine::new();
        let img = render_for_export(&doc, ExportFormat::Jpeg, true, &mut text).unwrap();
        let path = test_dir().join("opaque.jpg");
        codec::write_image(&path, ExportFormat::Jpeg, &img.pixmap, 82).unwrap();
        let decoded = codec::load_pixmap(&path).unwrap();
        let corner = px(&decoded, 0, 0);
        assert!(corner.0 > 235 && corner.1 > 235 && corner.2 > 235);
        assert_eq!(corner.3, 255);
        let center = px(&decoded, 100, 50);
        assert!(center.0 > center.1 + 40);
    }

    #[test]
    fn bmp_roundtrip() {
        let doc = fixed_doc();
        let mut text = TextEngine::new();
        let img = render_for_export(&doc, ExportFormat::Bmp, false, &mut text).unwrap();
        let path = test_dir().join("export.bmp");
        codec::write_image(&path, ExportFormat::Bmp, &img.pixmap, 100).unwrap();
        let decoded = codec::load_pixmap(&path).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (200, 100));
    }

    #[test]
    fn auto_crops_to_visible_bounds() {
        let mut doc = Document::new();
        doc.add(Layer::shape(
            0,
            ShapeKind::Rect,
            Point::new(10.0, 10.0),
            Point::new(50.0, 40.0),
            Rgba::TRANSPARENT,
            Some(Rgba::opaque(20, 120, 220)),
            0.5,
            0.0,
        ));
        let mut text = TextEngine::new();
        let img = render_for_export(&doc, ExportFormat::Png, true, &mut text).unwrap();
        assert_eq!((img.pixmap.width(), img.pixmap.height()), (40, 30));
    }

    #[test]
    fn text_annotation_exports() {
        let mut doc = fixed_doc();
        let mut text = TextEngine::new();
        let (w, h) = text.measure("Hello 拼图", 32.0, 400);
        doc.add_text("Hello 拼图".into(), Rgba::WHITE, 32.0, Rect::new(30.0, 30.0, w, h));
        let img = render_for_export(&doc, ExportFormat::Png, true, &mut text).unwrap();
        let count = img
            .pixmap
            .data()
            .chunks_exact(4)
            .filter(|p| p[3] > 0)
            .count();
        assert!(count > 500);
    }
}
