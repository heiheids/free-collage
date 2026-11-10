use std::sync::Arc;

use tiny_skia::Pixmap;

use super::geometry::{distance_to_polyline, point_in_ellipse, Point, Rect};
use super::style::Rgba;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Arrow,
    Border,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MosaicCell {
    pub rect: Rect,
    pub color: Rgba,
}

#[derive(Clone, Debug)]
pub enum LayerKind {
    Image {
        pixmap: Arc<Pixmap>,
    },
    Text {
        text: String,
        color: Rgba,
        font_size: f32,
        font_family: String,
        outline: Option<Rgba>,
        outline_width: f32,
    },
    Stroke {
        points: Vec<Point>,
        color: Rgba,
        width: f32,
    },
    Shape {
        kind: ShapeKind,
        a: Point,
        b: Point,
        stroke: Rgba,
        fill: Option<Rgba>,
        width: f32,
        radius: f32,
    },
    Mosaic {
        cell_size: f32,
        cells: Vec<MosaicCell>,
    },
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    pub rect: Rect,
    pub visible: bool,
    pub kind: LayerKind,
}

impl Layer {
    pub fn image(id: u64, name: impl Into<String>, pixmap: Arc<Pixmap>, rect: Rect) -> Self {
        Self {
            id,
            name: name.into(),
            rect,
            visible: true,
            kind: LayerKind::Image { pixmap },
        }
    }

    pub fn text(id: u64, text: impl Into<String>, color: Rgba, font_size: f32, rect: Rect) -> Self {
        Self::text_styled(
            id,
            text,
            color,
            font_size,
            "Microsoft YaHei UI",
            None,
            3.0,
            rect,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_styled(
        id: u64,
        text: impl Into<String>,
        color: Rgba,
        font_size: f32,
        font_family: impl Into<String>,
        outline: Option<Rgba>,
        outline_width: f32,
        rect: Rect,
    ) -> Self {
        Self {
            id,
            name: "text".into(),
            rect,
            visible: true,
            kind: LayerKind::Text {
                text: text.into(),
                color,
                font_size,
                font_family: font_family.into(),
                outline,
                outline_width,
            },
        }
    }

    pub fn stroke(id: u64, points: Vec<Point>, color: Rgba, width: f32) -> Self {
        let rect = stroke_bounds(&points, width);
        Self {
            id,
            name: "brush".into(),
            rect,
            visible: true,
            kind: LayerKind::Stroke { points, color, width },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape(
        id: u64,
        kind: ShapeKind,
        a: Point,
        b: Point,
        stroke: Rgba,
        fill: Option<Rgba>,
        width: f32,
        radius: f32,
    ) -> Self {
        let rect = Rect::from_points(a, b);
        Self {
            id,
            name: "shape".into(),
            rect,
            visible: true,
            kind: LayerKind::Shape {
                kind,
                a,
                b,
                stroke,
                fill,
                width,
                radius,
            },
        }
    }

    pub fn mosaic(id: u64, cell_size: f32) -> Self {
        Self {
            id,
            name: "mosaic".into(),
            rect: Rect::ZERO,
            visible: true,
            kind: LayerKind::Mosaic {
                cell_size: cell_size.max(2.0),
                cells: Vec::new(),
            },
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self.kind, LayerKind::Image { .. })
    }

    pub fn is_text(&self) -> bool {
        matches!(self.kind, LayerKind::Text { .. })
    }

    pub fn pixmap(&self) -> Option<&Arc<Pixmap>> {
        match &self.kind {
            LayerKind::Image { pixmap } => Some(pixmap),
            _ => None,
        }
    }

    pub fn pixmap_mut(&mut self) -> Option<&mut Pixmap> {
        match &mut self.kind {
            LayerKind::Image { pixmap } => Some(Arc::make_mut(pixmap)),
            _ => None,
        }
    }

    pub fn text_value(&self) -> Option<&str> {
        match &self.kind {
            LayerKind::Text { text, .. } => Some(text),
            _ => None,
        }
    }

    pub fn text_style(&self) -> Option<(Rgba, f32)> {
        match &self.kind {
            LayerKind::Text { color, font_size, .. } => Some((*color, *font_size)),
            _ => None,
        }
    }

    pub fn text_family(&self) -> Option<&str> {
        match &self.kind {
            LayerKind::Text { font_family, .. } => Some(font_family),
            _ => None,
        }
    }

    pub fn text_outline(&self) -> Option<Rgba> {
        match &self.kind {
            LayerKind::Text { outline, .. } => *outline,
            _ => None,
        }
    }

    pub fn text_outline_width(&self) -> Option<f32> {
        match &self.kind {
            LayerKind::Text { outline_width, .. } => Some(*outline_width),
            _ => None,
        }
    }

    pub fn set_text_style(
        &mut self,
        font_family: &str,
        outline: Option<Rgba>,
        outline_width: f32,
    ) {
        if let LayerKind::Text {
            font_family: family,
            outline: edge,
            outline_width: width,
            ..
        } = &mut self.kind
        {
            *family = font_family.to_string();
            *edge = outline;
            *width = outline_width.max(0.0);
        }
    }

    pub fn set_text_outline(&mut self, outline: Option<Rgba>) {
        if let LayerKind::Text { outline: edge, .. } = &mut self.kind {
            *edge = outline;
        }
    }

    pub fn set_text_outline_width(&mut self, width: f32) {
        if let LayerKind::Text { outline_width, .. } = &mut self.kind {
            *outline_width = width.max(0.0);
        }
    }

    pub fn set_text(&mut self, value: impl Into<String>, rect: Rect, font_size: f32) {
        if let LayerKind::Text { text, font_size: fs, .. } = &mut self.kind {
            *text = value.into();
            *fs = font_size.max(6.0);
            self.rect = rect;
        }
    }

    pub fn set_text_color(&mut self, color: Rgba) {
        if let LayerKind::Text { color: c, .. } = &mut self.kind {
            *c = color;
        }
    }

    pub fn set_shape_style(&mut self, stroke: Option<Rgba>, fill: Option<Option<Rgba>>, width: Option<f32>) {
        if let LayerKind::Shape {
            stroke: s,
            fill: f,
            width: w,
            ..
        } = &mut self.kind
        {
            if let Some(c) = stroke {
                *s = c;
            }
            if let Some(c) = fill {
                *f = c;
            }
            if let Some(v) = width {
                *w = v.max(0.5);
            }
        }
    }

    pub fn set_stroke_style(&mut self, color: Option<Rgba>, width: Option<f32>) {
        if let LayerKind::Stroke {
            color: c, width: w, ..
        } = &mut self.kind
        {
            if let Some(v) = color {
                *c = v;
            }
            if let Some(v) = width {
                *w = v.max(0.5);
            }
            if let LayerKind::Stroke { points, width, .. } = &self.kind {
                self.rect = stroke_bounds(points, *width);
            }
        }
    }

    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.rect = self.rect.offset(dx, dy);
        match &mut self.kind {
            LayerKind::Stroke { points, .. } => {
                for p in points.iter_mut() {
                    p.x += dx;
                    p.y += dy;
                }
            }
            LayerKind::Shape { a, b, .. } => {
                a.x += dx;
                a.y += dy;
                b.x += dx;
                b.y += dy;
            }
            LayerKind::Mosaic { cells, .. } => {
                for cell in cells.iter_mut() {
                    cell.rect = cell.rect.offset(dx, dy);
                }
            }
            _ => {}
        }
    }

    pub fn scale_from(&mut self, anchor: Point, sx: f32, sy: f32) {
        if !sx.is_finite() || !sy.is_finite() || sx.abs() < 1e-4 || sy.abs() < 1e-4 {
            return;
        }
        let scale_rect = |r: Rect| {
            Rect::from_ltrb(
                anchor.x + (r.x - anchor.x) * sx,
                anchor.y + (r.y - anchor.y) * sy,
                anchor.x + (r.right() - anchor.x) * sx,
                anchor.y + (r.bottom() - anchor.y) * sy,
            )
        };
        self.rect = scale_rect(self.rect);
        let avg = ((sx.abs() + sy.abs()) * 0.5).max(0.01);
        match &mut self.kind {
            LayerKind::Text {
                font_size,
                outline_width,
                ..
            } => {
                *font_size = (*font_size * avg).clamp(6.0, 800.0);
                *outline_width = (*outline_width * avg).clamp(0.0, 80.0);
            }
            LayerKind::Stroke { points, width, .. } => {
                for p in points.iter_mut() {
                    p.x = anchor.x + (p.x - anchor.x) * sx;
                    p.y = anchor.y + (p.y - anchor.y) * sy;
                }
                *width = (*width * avg).clamp(0.5, 400.0);
            }
            LayerKind::Shape { a, b, width, .. } => {
                a.x = anchor.x + (a.x - anchor.x) * sx;
                a.y = anchor.y + (a.y - anchor.y) * sy;
                b.x = anchor.x + (b.x - anchor.x) * sx;
                b.y = anchor.y + (b.y - anchor.y) * sy;
                *width = (*width * avg).clamp(0.5, 400.0);
            }
            LayerKind::Mosaic { cells, cell_size } => {
                for cell in cells.iter_mut() {
                    cell.rect = scale_rect(cell.rect);
                }
                *cell_size = (*cell_size * avg).clamp(2.0, 500.0);
            }
            LayerKind::Image { .. } => {}
        }
        if let LayerKind::Shape { a, b, .. } = self.kind {
            self.rect = Rect::from_points(a, b);
        }
    }

    pub fn erase_image_alpha(&mut self, p: Point, radius: f32) -> bool {
        let LayerKind::Image { pixmap } = &mut self.kind else {
            return false;
        };
        let rect = self.rect;
        if rect.w <= 0.0 || rect.h <= 0.0 || radius <= 0.0 {
            return false;
        }
        let pw = pixmap.width() as f32;
        let ph = pixmap.height() as f32;
        let cx = (p.x - rect.x) / rect.w * pw;
        let cy = (p.y - rect.y) / rect.h * ph;
        let rx = radius / rect.w * pw;
        let ry = radius / rect.h * ph;
        if rx <= 0.0 || ry <= 0.0 {
            return false;
        }
        let data = Arc::make_mut(pixmap).data_mut();
        let w = pw as i32;
        let h = ph as i32;
        let x0 = ((cx - rx).floor() as i32).max(0);
        let x1 = ((cx + rx).ceil() as i32).min(w - 1);
        let y0 = ((cy - ry).floor() as i32).max(0);
        let y1 = ((cy + ry).ceil() as i32).min(h - 1);
        let mut changed = false;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    let idx = ((y * w + x) * 4) as usize;
                    if data[idx + 3] != 0 {
                        data[idx] = 0;
                        data[idx + 1] = 0;
                        data[idx + 2] = 0;
                        data[idx + 3] = 0;
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    pub fn erase_mosaic(&mut self, p: Point, radius: f32) -> bool {
        let LayerKind::Mosaic { cells, .. } = &mut self.kind else {
            return false;
        };
        let before = cells.len();
        cells.retain(|c| c.rect.center().distance(p) > radius);
        if cells.len() != before {
            let mut rect = Rect::ZERO;
            for c in cells.iter() {
                rect = rect.union(c.rect);
            }
            self.rect = rect;
            true
        } else {
            false
        }
    }

    pub fn hit_test(&self, p: Point, tolerance: f32) -> bool {
        if !self.visible || !self.rect.inflate(tolerance).contains(p) {
            return false;
        }
        match &self.kind {
            LayerKind::Stroke { points, width, .. } => {
                distance_to_polyline(p, points) <= width * 0.5 + tolerance
            }
            LayerKind::Shape { kind, a, b, width, .. } => match kind {
                ShapeKind::Arrow => {
                    super::geometry::dist_to_segment(p, *a, *b) <= width * 0.5 + tolerance
                }
                ShapeKind::Ellipse => point_in_ellipse(p, self.rect.inflate(tolerance)),
                _ => true,
            },
            _ => true,
        }
    }

    pub fn image_sample(&self, p: Point) -> Option<Rgba> {
        let LayerKind::Image { pixmap } = &self.kind else {
            return None;
        };
        if self.rect.w <= 0.0 || self.rect.h <= 0.0 || !self.rect.contains(p) {
            return None;
        }
        let u = ((p.x - self.rect.x) / self.rect.w * pixmap.width() as f32).floor() as i32;
        let v = ((p.y - self.rect.y) / self.rect.h * pixmap.height() as f32).floor() as i32;
        if u < 0 || v < 0 || u >= pixmap.width() as i32 || v >= pixmap.height() as i32 {
            return None;
        }
        let px = pixmap.pixel(u as u32, v as u32)?;
        let a = px.alpha();
        if a == 0 {
            return None;
        }
        let a32 = a as u32;
        let un = |c: u8| ((c as u32 * 255 + a32 / 2) / a32).min(255) as u8;
        Some(Rgba::new(un(px.red()), un(px.green()), un(px.blue()), a))
    }

    pub fn sample(&self, p: Point) -> Option<Rgba> {
        if !self.visible {
            return None;
        }
        match &self.kind {
            LayerKind::Image { .. } => self.image_sample(p),
            LayerKind::Text { color, .. } => {
                if self.rect.contains(p) {
                    Some(*color)
                } else {
                    None
                }
            }
            LayerKind::Stroke { points, color, width } => {
                if distance_to_polyline(p, points) <= width * 0.5 {
                    Some(*color)
                } else {
                    None
                }
            }
            LayerKind::Shape { kind, a, b, stroke, fill, width, .. } => match kind {
                ShapeKind::Ellipse => {
                    if point_in_ellipse(p, self.rect) {
                        fill.or(Some(*stroke))
                    } else {
                        None
                    }
                }
                ShapeKind::Rect | ShapeKind::Border => {
                    if self.rect.contains(p) {
                        fill.or(Some(*stroke))
                    } else {
                        None
                    }
                }
                ShapeKind::Arrow => {
                    if super::geometry::dist_to_segment(p, *a, *b) <= width * 0.5 {
                        Some(*stroke)
                    } else {
                        None
                    }
                }
            },
            LayerKind::Mosaic { cells, .. } => cells
                .iter()
                .rev()
                .find(|c| c.rect.contains(p))
                .map(|c| c.color),
        }
    }
}

fn stroke_bounds(points: &[Point], width: f32) -> Rect {
    if points.is_empty() {
        return Rect::ZERO;
    }
    let mut r = Rect::from_points(points[0], points[0]);
    for p in points {
        r = r.union(Rect::from_points(*p, *p));
    }
    r.inflate(width * 0.5 + 1.0)
}
