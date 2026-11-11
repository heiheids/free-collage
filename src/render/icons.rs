use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect as SkRect, Stroke,
    Transform,
};

use crate::model::geometry::Rect;
use crate::model::style::Rgba;

pub fn sk_rect(r: Rect) -> Option<SkRect> {
    SkRect::from_xywh(r.x, r.y, r.w.max(0.01), r.h.max(0.01))
}

pub fn fill_rect(pixmap: &mut Pixmap, rect: Rect, color: Rgba) {
    if rect.w <= 0.0 || rect.h <= 0.0 || color.a == 0 {
        return;
    }
    let Some(r) = sk_rect(rect) else { return };
    let mut paint = Paint::default();
    paint.set_color(color.to_color());
    paint.anti_alias = true;
    pixmap.fill_rect(r, &paint, Transform::identity(), None);
}

pub fn round_rect_path(rect: Rect, radius: f32) -> Option<Path> {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return None;
    }
    let r = radius.min(rect.w * 0.5).min(rect.h * 0.5).max(0.0);
    let mut pb = PathBuilder::new();
    pb.move_to(rect.x + r, rect.y);
    pb.line_to(rect.right() - r, rect.y);
    pb.quad_to(rect.right(), rect.y, rect.right(), rect.y + r);
    pb.line_to(rect.right(), rect.bottom() - r);
    pb.quad_to(rect.right(), rect.bottom(), rect.right() - r, rect.bottom());
    pb.line_to(rect.x + r, rect.bottom());
    pb.quad_to(rect.x, rect.bottom(), rect.x, rect.bottom() - r);
    pb.line_to(rect.x, rect.y + r);
    pb.quad_to(rect.x, rect.y, rect.x + r, rect.y);
    pb.close();
    pb.finish()
}

pub fn fill_round_rect(pixmap: &mut Pixmap, rect: Rect, radius: f32, color: Rgba) {
    if color.a == 0 {
        return;
    }
    let Some(path) = round_rect_path(rect, radius) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(color.to_color());
    paint.anti_alias = true;
    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

pub fn stroke_round_rect(
    pixmap: &mut Pixmap,
    rect: Rect,
    radius: f32,
    color: Rgba,
    width: f32,
) {
    let Some(path) = round_rect_path(rect, radius) else {
        return;
    };
    stroke_path(pixmap, &path, color, width);
}

pub fn stroke_path(pixmap: &mut Pixmap, path: &Path, color: Rgba, width: f32) {
    if color.a == 0 || width <= 0.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_color(color.to_color());
    paint.anti_alias = true;
    let stroke = Stroke {
        width: width.max(0.1),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    pixmap.stroke_path(path, &paint, &stroke, Transform::identity(), None);
}

pub fn fill_path(pixmap: &mut Pixmap, path: &Path, color: Rgba) {
    if color.a == 0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_color(color.to_color());
    paint.anti_alias = true;
    pixmap.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
}

pub fn line(pixmap: &mut Pixmap, a: crate::model::Point, b: crate::model::Point, color: Rgba, width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(a.x, a.y);
    pb.line_to(b.x, b.y);
    if let Some(path) = pb.finish() {
        stroke_path(pixmap, &path, color, width);
    }
}

pub fn dashed_rect(
    pixmap: &mut Pixmap,
    rect: Rect,
    color: Rgba,
    width: f32,
    dash: f32,
    gap: f32,
) {
    if rect.w <= 0.0 || rect.h <= 0.0 || color.a == 0 || width <= 0.0 {
        return;
    }
    let step = (dash + gap).max(1.0);
    let mut x = rect.x;
    while x < rect.right() {
        let end = (x + dash).min(rect.right());
        line(
            pixmap,
            crate::model::Point::new(x, rect.y),
            crate::model::Point::new(end, rect.y),
            color,
            width,
        );
        line(
            pixmap,
            crate::model::Point::new(x, rect.bottom()),
            crate::model::Point::new(end, rect.bottom()),
            color,
            width,
        );
        x += step;
    }
    let mut y = rect.y;
    while y < rect.bottom() {
        let end = (y + dash).min(rect.bottom());
        line(
            pixmap,
            crate::model::Point::new(rect.x, y),
            crate::model::Point::new(rect.x, end),
            color,
            width,
        );
        line(
            pixmap,
            crate::model::Point::new(rect.right(), y),
            crate::model::Point::new(rect.right(), end),
            color,
            width,
        );
        y += step;
    }
}

pub fn circle(pixmap: &mut Pixmap, center: crate::model::Point, radius: f32, color: Rgba) {
    let mut pb = PathBuilder::new();
    pb.push_circle(center.x, center.y, radius.max(0.1));
    if let Some(path) = pb.finish() {
        fill_path(pixmap, &path, color);
    }
}

pub fn ring(pixmap: &mut Pixmap, center: crate::model::Point, radius: f32, color: Rgba, width: f32) {
    let mut pb = PathBuilder::new();
    pb.push_circle(center.x, center.y, radius.max(0.1));
    if let Some(path) = pb.finish() {
        stroke_path(pixmap, &path, color, width);
    }
}

pub fn shadow(pixmap: &mut Pixmap, rect: Rect, radius: f32) {
    let layers = [
        (5.0, 14.0, Rgba::new(20, 31, 50, 8)),
        (2.0, 7.0, Rgba::new(20, 31, 50, 12)),
        (0.0, 2.0, Rgba::new(20, 31, 50, 16)),
    ];
    for (dy, spread, color) in layers {
        let r = Rect::new(
            rect.x - spread,
            rect.y - spread + dy,
            rect.w + spread * 2.0,
            rect.h + spread * 2.0,
        );
        fill_round_rect(pixmap, r, radius + spread * 0.5, color);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Select,
    Open,
    Paste,
    Delete,
    Clear,
    Undo,
    Save,
    Magnet,
    Crop,
    Text,
    Brush,
    Mosaic,
    Eraser,
    Arrow,
    Border,
    Rect,
    Ellipse,
    Info,
    Globe,
    ChevronDown,
    Plus,
    Minus,
    Close,
    Maximize,
    Restore,
    Minimize,
    Check,
    Folder,
    Spark,
}

fn icon_path(icon: Icon, r: Rect) -> Option<Path> {
    let x = |v: f32| r.x + v * r.w;
    let y = |v: f32| r.y + v * r.h;
    let mut pb = PathBuilder::new();
    match icon {
        Icon::Select => {
            pb.move_to(x(0.26), y(0.14));
            pb.line_to(x(0.72), y(0.55));
            pb.line_to(x(0.50), y(0.59));
            pb.line_to(x(0.65), y(0.84));
            pb.line_to(x(0.53), y(0.90));
            pb.line_to(x(0.39), y(0.65));
            pb.line_to(x(0.24), y(0.79));
            pb.close();
        }
        Icon::Open => {
            pb.move_to(x(0.10), y(0.34));
            pb.line_to(x(0.10), y(0.78));
            pb.line_to(x(0.90), y(0.78));
            pb.line_to(x(0.90), y(0.38));
            pb.line_to(x(0.54), y(0.38));
            pb.line_to(x(0.44), y(0.27));
            pb.line_to(x(0.18), y(0.27));
            pb.line_to(x(0.10), y(0.34));
            pb.close();
        }
        Icon::Paste => {
            pb.move_to(x(0.29), y(0.18));
            pb.line_to(x(0.21), y(0.18));
            pb.line_to(x(0.21), y(0.90));
            pb.line_to(x(0.79), y(0.90));
            pb.line_to(x(0.79), y(0.18));
            pb.line_to(x(0.71), y(0.18));
            pb.move_to(x(0.38), y(0.10));
            pb.line_to(x(0.62), y(0.10));
            pb.line_to(x(0.62), y(0.24));
            pb.line_to(x(0.38), y(0.24));
            pb.close();
            pb.move_to(x(0.39), y(0.62));
            pb.line_to(x(0.61), y(0.62));
            pb.move_to(x(0.50), y(0.48));
            pb.line_to(x(0.50), y(0.69));
        }
        Icon::Delete => {
            pb.move_to(x(0.25), y(0.28));
            pb.line_to(x(0.75), y(0.28));
            pb.move_to(x(0.33), y(0.28));
            pb.line_to(x(0.36), y(0.88));
            pb.line_to(x(0.64), y(0.88));
            pb.line_to(x(0.67), y(0.28));
            pb.move_to(x(0.40), y(0.20));
            pb.line_to(x(0.60), y(0.20));
            pb.move_to(x(0.44), y(0.40));
            pb.line_to(x(0.46), y(0.78));
            pb.move_to(x(0.56), y(0.40));
            pb.line_to(x(0.54), y(0.78));
        }
        Icon::Clear => {
            pb.push_rect(
                SkRect::from_xywh(x(0.20), y(0.34), r.w * 0.44, r.h * 0.48).unwrap(),
            );
            pb.move_to(x(0.58), y(0.18));
            pb.line_to(x(0.84), y(0.18));
            pb.line_to(x(0.84), y(0.54));
            pb.move_to(x(0.62), y(0.46));
            pb.line_to(x(0.82), y(0.66));
            pb.move_to(x(0.82), y(0.46));
            pb.line_to(x(0.62), y(0.66));
        }
        Icon::Undo => {
            pb.move_to(x(0.30), y(0.34));
            pb.cubic_to(x(0.48), y(0.14), x(0.78), y(0.24), x(0.80), y(0.50));
            pb.cubic_to(x(0.81), y(0.72), x(0.62), y(0.84), x(0.40), y(0.78));
            pb.move_to(x(0.30), y(0.34));
            pb.line_to(x(0.30), y(0.16));
            pb.move_to(x(0.30), y(0.34));
            pb.line_to(x(0.48), y(0.34));
        }
        Icon::Save => {
            pb.move_to(x(0.19), y(0.15));
            pb.line_to(x(0.72), y(0.15));
            pb.line_to(x(0.84), y(0.27));
            pb.line_to(x(0.84), y(0.85));
            pb.line_to(x(0.16), y(0.85));
            pb.line_to(x(0.16), y(0.15));
            pb.close();
            pb.move_to(x(0.33), y(0.15));
            pb.line_to(x(0.33), y(0.42));
            pb.line_to(x(0.68), y(0.42));
            pb.line_to(x(0.68), y(0.15));
            pb.move_to(x(0.30), y(0.62));
            pb.line_to(x(0.70), y(0.62));
            pb.line_to(x(0.70), y(0.82));
            pb.line_to(x(0.30), y(0.82));
            pb.close();
        }
        Icon::Magnet => {
            // U-shaped horseshoe magnet with two pole caps at the top.
            pb.move_to(x(0.28), y(0.18));
            pb.line_to(x(0.28), y(0.50));
            pb.cubic_to(x(0.28), y(0.78), x(0.72), y(0.78), x(0.72), y(0.50));
            pb.line_to(x(0.72), y(0.18));
            pb.move_to(x(0.19), y(0.18));
            pb.line_to(x(0.36), y(0.18));
            pb.move_to(x(0.64), y(0.18));
            pb.line_to(x(0.81), y(0.18));
        }
        Icon::Crop => {
            // Crop marks: two overlapping L-shaped brackets.
            pb.move_to(x(0.18), y(0.36));
            pb.line_to(x(0.18), y(0.82));
            pb.line_to(x(0.64), y(0.82));
            pb.move_to(x(0.36), y(0.18));
            pb.line_to(x(0.82), y(0.18));
            pb.line_to(x(0.82), y(0.64));
        }
        Icon::Text => {
            pb.move_to(x(0.22), y(0.25));
            pb.line_to(x(0.78), y(0.25));
            pb.move_to(x(0.50), y(0.25));
            pb.line_to(x(0.50), y(0.82));
            pb.move_to(x(0.36), y(0.82));
            pb.line_to(x(0.64), y(0.82));
        }
        Icon::Brush => {
            pb.move_to(x(0.22), y(0.78));
            pb.cubic_to(x(0.24), y(0.58), x(0.44), y(0.50), x(0.58), y(0.34));
            pb.line_to(x(0.76), y(0.18));
            pb.line_to(x(0.84), y(0.26));
            pb.line_to(x(0.66), y(0.44));
            pb.cubic_to(x(0.50), y(0.58), x(0.42), y(0.76), x(0.22), y(0.78));
            pb.close();
        }
        Icon::Mosaic => {
            for (cx, cy) in [(0.22, 0.22), (0.48, 0.22), (0.74, 0.22), (0.22, 0.48), (0.48, 0.48), (0.74, 0.48), (0.22, 0.74), (0.48, 0.74), (0.74, 0.74)] {
                pb.push_rect(
                    SkRect::from_xywh(x(cx) - r.w * 0.10, y(cy) - r.h * 0.10, r.w * 0.20, r.h * 0.20)
                        .unwrap(),
                );
            }
        }
        Icon::Eraser => {
            pb.move_to(x(0.24), y(0.68));
            pb.line_to(x(0.50), y(0.24));
            pb.line_to(x(0.74), y(0.38));
            pb.line_to(x(0.54), y(0.76));
            pb.line_to(x(0.34), y(0.76));
            pb.close();
            pb.move_to(x(0.30), y(0.72));
            pb.line_to(x(0.80), y(0.72));
        }
        Icon::Arrow => {
            pb.move_to(x(0.18), y(0.82));
            pb.line_to(x(0.78), y(0.22));
            pb.move_to(x(0.50), y(0.22));
            pb.line_to(x(0.78), y(0.22));
            pb.line_to(x(0.78), y(0.50));
        }
        Icon::Border => {
            let r2 = Rect::new(x(0.16), y(0.20), r.w * 0.68, r.h * 0.58);
            return round_rect_path(r2, r.w.min(r.h) * 0.10);
        }
        Icon::Rect => {
            pb.push_rect(
                SkRect::from_xywh(x(0.18), y(0.22), r.w * 0.64, r.h * 0.56).unwrap(),
            );
        }
        Icon::Ellipse => {
            pb.push_circle(x(0.50), y(0.50), r.w.min(r.h) * 0.33);
        }
        Icon::Info => {
            pb.push_circle(x(0.50), y(0.50), r.w.min(r.h) * 0.36);
            pb.move_to(x(0.50), y(0.42));
            pb.line_to(x(0.50), y(0.66));
            pb.move_to(x(0.50), y(0.31));
            pb.line_to(x(0.50), y(0.32));
        }
        Icon::Globe => {
            pb.push_circle(x(0.50), y(0.50), r.w.min(r.h) * 0.36);
            pb.move_to(x(0.14), y(0.50));
            pb.line_to(x(0.86), y(0.50));
            pb.move_to(x(0.50), y(0.14));
            pb.cubic_to(x(0.24), y(0.50), x(0.24), y(0.50), x(0.50), y(0.86));
            pb.cubic_to(x(0.76), y(0.50), x(0.76), y(0.50), x(0.50), y(0.14));
        }
        Icon::ChevronDown => {
            pb.move_to(x(0.25), y(0.40));
            pb.line_to(x(0.50), y(0.64));
            pb.line_to(x(0.75), y(0.40));
        }
        Icon::Plus => {
            pb.move_to(x(0.50), y(0.22));
            pb.line_to(x(0.50), y(0.78));
            pb.move_to(x(0.22), y(0.50));
            pb.line_to(x(0.78), y(0.50));
        }
        Icon::Minus => {
            pb.move_to(x(0.22), y(0.50));
            pb.line_to(x(0.78), y(0.50));
        }
        Icon::Close => {
            pb.move_to(x(0.28), y(0.28));
            pb.line_to(x(0.72), y(0.72));
            pb.move_to(x(0.72), y(0.28));
            pb.line_to(x(0.28), y(0.72));
        }
        Icon::Maximize => {
            pb.push_rect(
                SkRect::from_xywh(x(0.28), y(0.28), r.w * 0.44, r.h * 0.44).unwrap(),
            );
        }
        Icon::Restore => {
            pb.push_rect(
                SkRect::from_xywh(x(0.24), y(0.34), r.w * 0.40, r.h * 0.38).unwrap(),
            );
            pb.move_to(x(0.36), y(0.30));
            pb.line_to(x(0.76), y(0.30));
            pb.line_to(x(0.76), y(0.68));
            pb.line_to(x(0.64), y(0.68));
        }
        Icon::Minimize => {
            pb.move_to(x(0.28), y(0.54));
            pb.line_to(x(0.72), y(0.54));
        }
        Icon::Check => {
            pb.move_to(x(0.22), y(0.52));
            pb.line_to(x(0.42), y(0.72));
            pb.line_to(x(0.80), y(0.28));
        }
        Icon::Folder => {
            pb.move_to(x(0.12), y(0.30));
            pb.line_to(x(0.12), y(0.78));
            pb.line_to(x(0.88), y(0.78));
            pb.line_to(x(0.88), y(0.38));
            pb.line_to(x(0.52), y(0.38));
            pb.line_to(x(0.42), y(0.26));
            pb.line_to(x(0.12), y(0.26));
            pb.close();
        }
        Icon::Spark => {
            pb.move_to(x(0.50), y(0.14));
            pb.cubic_to(x(0.54), y(0.40), x(0.60), y(0.46), x(0.86), y(0.50));
            pb.cubic_to(x(0.60), y(0.54), x(0.54), y(0.60), x(0.50), y(0.86));
            pb.cubic_to(x(0.46), y(0.60), x(0.40), y(0.54), x(0.14), y(0.50));
            pb.cubic_to(x(0.40), y(0.46), x(0.46), y(0.40), x(0.50), y(0.14));
            pb.close();
        }
    }
    pb.finish()
}

pub fn draw_icon(pixmap: &mut Pixmap, icon: Icon, rect: Rect, color: Rgba, width: f32) {
    match icon {
        Icon::Select
        | Icon::Mosaic
        | Icon::Check
        | Icon::Spark
        | Icon::Maximize
        | Icon::Restore => {
            if let Some(path) = icon_path(icon, rect) {
                if icon == Icon::Select || icon == Icon::Spark {
                    fill_path(pixmap, &path, color);
                } else if icon == Icon::Mosaic {
                    let cell = rect.w * 0.20;
                    for (i, cx) in [0.22f32, 0.48, 0.74].iter().enumerate() {
                        for (j, cy) in [0.22f32, 0.48, 0.74].iter().enumerate() {
                            let rr = Rect::new(
                                rect.x + cx * rect.w - cell * 0.5,
                                rect.y + cy * rect.h - cell * 0.5,
                                cell,
                                cell,
                            );
                            fill_round_rect(pixmap, rr, cell * 0.18, if (i + j) % 2 == 0 { color } else { color.with_alpha(120) });
                        }
                    }
                } else {
                    stroke_path(pixmap, &path, color, width);
                }
            }
        }
        _ => {
            if let Some(path) = icon_path(icon, rect) {
                stroke_path(pixmap, &path, color, width);
            }
        }
    }
}

pub fn draw_logo(pixmap: &mut Pixmap, rect: Rect) {
    let bg = rect;
    fill_round_rect(pixmap, bg, rect.w * 0.24, Rgba::opaque(15, 118, 110));
    let card1 = Rect::new(rect.x + rect.w * 0.20, rect.y + rect.h * 0.22, rect.w * 0.34, rect.h * 0.28);
    fill_round_rect(pixmap, card1, rect.w * 0.05, Rgba::WHITE);
    let card2 = Rect::new(rect.x + rect.w * 0.48, rect.y + rect.h * 0.33, rect.w * 0.32, rect.h * 0.24);
    fill_round_rect(pixmap, card2, rect.w * 0.05, Rgba::opaque(253, 230, 138));
    let card3 = Rect::new(rect.x + rect.w * 0.28, rect.y + rect.h * 0.56, rect.w * 0.44, rect.h * 0.22);
    fill_round_rect(pixmap, card3, rect.w * 0.05, Rgba::WHITE);
    circle(pixmap, crate::model::Point::new(rect.x + rect.w * 0.75, rect.y + rect.h * 0.72), rect.w * 0.07, Rgba::opaque(251, 113, 133));
}
