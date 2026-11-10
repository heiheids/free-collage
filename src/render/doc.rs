use std::sync::Arc;

use tiny_skia::{
    FillRule, Paint, Path, PathBuilder, Pixmap, Transform,
};

use crate::model::geometry::{Point, Rect};
use crate::model::layer::{LayerKind, ShapeKind};
use crate::model::style::Rgba;
use crate::model::Document;
use crate::platform::gdi::TextEngine;
use crate::render::icons::{fill_path, fill_rect, fill_round_rect, round_rect_path, stroke_path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    Transparent,
    White,
    Checker,
}

pub fn doc_to_screen(p: Point, zoom: f32, pan: Point) -> Point {
    Point::new(p.x * zoom + pan.x, p.y * zoom + pan.y)
}

pub fn page_screen_rect(page: Rect, zoom: f32, pan: Point) -> Rect {
    Rect::new(page.x * zoom + pan.x, page.y * zoom + pan.y, page.w * zoom, page.h * zoom)
}

pub fn fit_view(page: Option<Rect>, viewport: Rect, padding: f32) -> (f32, Point) {
    let Some(page) = page else {
        return (1.0, viewport.center());
    };
    if page.w <= 0.0 || page.h <= 0.0 {
        return (1.0, viewport.center());
    }
    let avail_w = (viewport.w - padding * 2.0).max(40.0);
    let avail_h = (viewport.h - padding * 2.0).max(40.0);
    let zoom = (avail_w / page.w).min(avail_h / page.h).clamp(0.02, 8.0);
    let pan = Point::new(
        viewport.center().x - page.center().x * zoom,
        viewport.center().y - page.center().y * zoom,
    );
    (zoom, pan)
}

#[allow(clippy::too_many_arguments)]
pub fn render_document(
    doc: &Document,
    page: Rect,
    zoom: f32,
    offset: Point,
    target_w: u32,
    target_h: u32,
    background: Background,
    text: &mut TextEngine,
    editing: Option<u64>,
    checker_size: f32,
    smooth: bool,
) -> Option<Pixmap> {
    if target_w == 0 || target_h == 0 || zoom <= 0.0 {
        return None;
    }
    let mut pixmap = Pixmap::new(target_w, target_h)?;
    match background {
        Background::Transparent => {}
        Background::White => pixmap.fill(Rgba::WHITE.to_color()),
        Background::Checker => draw_checker(&mut pixmap, checker_size.max(4.0)),
    }

    let sx = |v: f32| v * zoom + offset.x;
    let sy = |v: f32| v * zoom + offset.y;

    for layer in &doc.layers {
        if !layer.visible || editing == Some(layer.id) {
            continue;
        }
        match &layer.kind {
            LayerKind::Image { pixmap: img } => {
                if layer.rect.w <= 0.0 || layer.rect.h <= 0.0 {
                    continue;
                }
                let dw = layer.rect.w * zoom;
                let dh = layer.rect.h * zoom;
                if smooth || dw * dh < 250_000.0 {
                    draw_image_scaled_smooth(
                        &mut pixmap,
                        img.as_ref(),
                        sx(layer.rect.x),
                        sy(layer.rect.y),
                        dw,
                        dh,
                    );
                } else {
                    draw_image_scaled(
                        &mut pixmap,
                        img.as_ref(),
                        sx(layer.rect.x),
                        sy(layer.rect.y),
                        dw,
                        dh,
                    );
                }
            }
            LayerKind::Text {
                text: value,
                color,
                font_size,
                font_family,
                outline,
                outline_width,
            } => {
                text.draw_family(
                    &mut pixmap,
                    sx(layer.rect.x),
                    sy(layer.rect.y),
                    value,
                    (*font_size * zoom).max(2.0),
                    400,
                    *color,
                    1.0,
                    font_family,
                    *outline,
                    (*outline_width * zoom).max(0.0),
                );
            }
            LayerKind::Stroke { points, color, width } => {
                if points.is_empty() {
                    continue;
                }
                let mut pb = PathBuilder::new();
                pb.move_to(sx(points[0].x), sy(points[0].y));
                for p in &points[1..] {
                    pb.line_to(sx(p.x), sy(p.y));
                }
                if let Some(path) = pb.finish() {
                    stroke_path(&mut pixmap, &path, *color, (*width * zoom).max(0.5));
                }
            }
            LayerKind::Shape { kind, a, b, stroke, fill, width, radius } => {
                let r = Rect::new(sx(layer.rect.x), sy(layer.rect.y), layer.rect.w * zoom, layer.rect.h * zoom);
                match kind {
                    ShapeKind::Rect => {
                        if let Some(path) = rect_path(r) {
                            if let Some(f) = fill {
                                fill_path(&mut pixmap, &path, *f);
                            }
                            stroke_path(&mut pixmap, &path, *stroke, (*width * zoom).max(0.5));
                        }
                    }
                    ShapeKind::Border => {
                        if let Some(path) = round_rect_path(r, *radius * zoom) {
                            stroke_path(&mut pixmap, &path, *stroke, (*width * zoom).max(0.5));
                        }
                    }
                    ShapeKind::Ellipse => {
                        if let Some(path) = oval_path(r) {
                            if let Some(f) = fill {
                                fill_path(&mut pixmap, &path, *f);
                            }
                            stroke_path(&mut pixmap, &path, *stroke, (*width * zoom).max(0.5));
                        }
                    }
                    ShapeKind::Arrow => {
                        draw_arrow(
                            &mut pixmap,
                            Point::new(sx(a.x), sy(a.y)),
                            Point::new(sx(b.x), sy(b.y)),
                            *stroke,
                            (*width * zoom).max(1.0),
                        );
                    }
                }
            }
            LayerKind::Mosaic { cells, .. } => {
                for cell in cells {
                    let r = Rect::new(
                        sx(cell.rect.x),
                        sy(cell.rect.y),
                        cell.rect.w * zoom,
                        cell.rect.h * zoom,
                    );
                    fill_rect(&mut pixmap, r, cell.color);
                }
            }
        }
    }
    Some(pixmap)
}

pub fn render_page_view(
    doc: &Document,
    page: Rect,
    zoom: f32,
    pan: Point,
    viewport: Rect,
    background: Background,
    text: &mut TextEngine,
    editing: Option<u64>,
    checker_size: f32,
) -> Option<(Pixmap, Rect)> {
    let page_screen = page_screen_rect(page, zoom, pan);
    let visible = page_screen.intersect(viewport);
    if visible.is_empty() {
        return None;
    }
    let left = visible.x.floor();
    let top = visible.y.floor();
    let right = visible.right().ceil();
    let bottom = visible.bottom().ceil();
    let target_w = (right - left).max(1.0) as u32;
    let target_h = (bottom - top).max(1.0) as u32;
    let offset = Point::new(pan.x - left, pan.y - top);
    let pixmap = render_document(
        doc,
        page,
        zoom,
        offset,
        target_w,
        target_h,
        background,
        text,
        editing,
        checker_size,
        false,
    )?;
    Some((pixmap, Rect::new(left, top, target_w as f32, target_h as f32)))
}

pub fn crop_transparent(pixmap: Pixmap) -> Option<Pixmap> {
    let w = pixmap.width();
    let h = pixmap.height();
    let data = pixmap.data();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for y in 0..h {
        for x in 0..w {
            if data[((y * w + x) * 4 + 3) as usize] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return None;
    }
    if min_x == 0 && min_y == 0 && max_x + 1 == w && max_y + 1 == h {
        return Some(pixmap);
    }
    let cw = max_x - min_x + 1;
    let ch = max_y - min_y + 1;
    let mut out = Pixmap::new(cw, ch)?;
    for y in 0..ch {
        let src_start = (((min_y + y) * w + min_x) * 4) as usize;
        let dst_start = ((y * cw) * 4) as usize;
        out.data_mut()[dst_start..dst_start + (cw * 4) as usize]
            .copy_from_slice(&data[src_start..src_start + (cw * 4) as usize]);
    }
    Some(out)
}

fn rect_path(r: Rect) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.push_rect(tiny_skia::Rect::from_xywh(r.x, r.y, r.w.max(0.01), r.h.max(0.01))?);
    pb.finish()
}

fn oval_path(r: Rect) -> Option<Path> {
    let rx = r.w * 0.5;
    let ry = r.h * 0.5;
    let cx = r.x + rx;
    let cy = r.y + ry;
    let k = 0.552_284_8;
    let mut pb = PathBuilder::new();
    pb.move_to(cx + rx, cy);
    pb.cubic_to(cx + rx, cy + ry * k, cx + rx * k, cy + ry, cx, cy + ry);
    pb.cubic_to(cx - rx * k, cy + ry, cx - rx, cy + ry * k, cx - rx, cy);
    pb.cubic_to(cx - rx, cy - ry * k, cx - rx * k, cy - ry, cx, cy - ry);
    pb.cubic_to(cx + rx * k, cy - ry, cx + rx, cy - ry * k, cx + rx, cy);
    pb.close();
    pb.finish()
}

fn draw_arrow(pixmap: &mut Pixmap, a: Point, b: Point, color: Rgba, width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(a.x, a.y);
    pb.line_to(b.x, b.y);
    if let Some(path) = pb.finish() {
        stroke_path(pixmap, &path, color, width);
    }
    let angle = (b.y - a.y).atan2(b.x - a.x);
    let size = (width * 4.5).max(9.0);
    let mut head = PathBuilder::new();
    head.move_to(b.x, b.y);
    head.line_to(
        b.x - angle.cos() * size + angle.sin() * size * 0.55,
        b.y - angle.sin() * size - angle.cos() * size * 0.55,
    );
    head.move_to(b.x, b.y);
    head.line_to(
        b.x - angle.cos() * size - angle.sin() * size * 0.55,
        b.y - angle.sin() * size + angle.cos() * size * 0.55,
    );
    if let Some(path) = head.finish() {
        stroke_path(pixmap, &path, color, width);
    }
}

fn draw_image_scaled_smooth(
    dst: &mut Pixmap,
    src: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    if sw < 1 || sh < 1 || dw <= 0.0 || dh <= 0.0 {
        return;
    }
    let pw = dst.width() as i32;
    let ph = dst.height() as i32;
    let x0 = dx.floor().max(0.0) as i32;
    let y0 = dy.floor().max(0.0) as i32;
    let x1 = (dx + dw).ceil().min(pw as f32) as i32;
    let y1 = (dy + dh).ceil().min(ph as f32) as i32;
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let src_data = src.data();
    let sx_scale = sw as f32 / dw;
    let sy_scale = sh as f32 / dh;
    let xs: Vec<(i32, i32, f32)> = (x0..x1)
        .map(|px| {
            let u = (((px as f32 + 0.5) - dx) * sx_scale - 0.5)
                .clamp(0.0, (sw - 1) as f32);
            let x0i = u.floor() as i32;
            (x0i, (x0i + 1).min(sw - 1), u - x0i as f32)
        })
        .collect();
    let dst_data = dst.data_mut();
    for py in y0..y1 {
        let v = (((py as f32 + 0.5) - dy) * sy_scale - 0.5)
            .clamp(0.0, (sh - 1) as f32);
        let y0i = v.floor() as i32;
        let y1i = (y0i + 1).min(sh - 1);
        let fy = v - y0i as f32;
        for (idx, px) in (x0..x1).enumerate() {
            let (x0i, x1i, fx) = xs[idx];
            let i00 = ((y0i * sw + x0i) * 4) as usize;
            let i10 = ((y0i * sw + x1i) * 4) as usize;
            let i01 = ((y1i * sw + x0i) * 4) as usize;
            let i11 = ((y1i * sw + x1i) * 4) as usize;
            let w00 = (1.0 - fx) * (1.0 - fy);
            let w10 = fx * (1.0 - fy);
            let w01 = (1.0 - fx) * fy;
            let w11 = fx * fy;
            let mut sample = [0u8; 4];
            for c in 0..4 {
                sample[c] = (src_data[i00 + c] as f32 * w00
                    + src_data[i10 + c] as f32 * w10
                    + src_data[i01 + c] as f32 * w01
                    + src_data[i11 + c] as f32 * w11)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            let a = sample[3] as u32;
            if a == 0 {
                continue;
            }
            let di = ((py * pw + px) * 4) as usize;
            if a == 255 {
                dst_data[di..di + 4].copy_from_slice(&sample);
                continue;
            }
            let inv = 255 - a;
            for c in 0..3 {
                dst_data[di + c] =
                    (sample[c] as u32 + dst_data[di + c] as u32 * inv / 255).min(255) as u8;
            }
            dst_data[di + 3] =
                (a + dst_data[di + 3] as u32 * inv / 255).min(255) as u8;
        }
    }
}

fn draw_image_scaled(
    dst: &mut Pixmap,
    src: &Pixmap,
    dx: f32,
    dy: f32,
    dw: f32,
    dh: f32,
) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    if sw < 1 || sh < 1 || dw <= 0.0 || dh <= 0.0 {
        return;
    }
    let pw = dst.width() as i32;
    let ph = dst.height() as i32;
    let x0 = dx.floor().max(0.0) as i32;
    let y0 = dy.floor().max(0.0) as i32;
    let x1 = (dx + dw).ceil().min(pw as f32) as i32;
    let y1 = (dy + dh).ceil().min(ph as f32) as i32;
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let src_data = src.data();
    let dst_data = dst.data_mut();
    let sx_scale = sw as f32 / dw;
    let sy_scale = sh as f32 / dh;
    let xs: Vec<u32> = (x0..x1)
        .map(|px| {
            ((((px as f32 + 0.5) - dx) * sx_scale).floor() as i32)
                .clamp(0, sw - 1) as u32
        })
        .collect();
    for py in y0..y1 {
        let syi = ((((py as f32 + 0.5) - dy) * sy_scale).floor() as i32).clamp(0, sh - 1);
        let src_row = syi * sw;
        for (idx, px) in (x0..x1).enumerate() {
            let si = ((src_row + xs[idx] as i32) * 4) as usize;
            let a = src_data[si + 3] as u32;
            if a == 0 {
                continue;
            }
            let di = ((py * pw + px) * 4) as usize;
            if a == 255 {
                dst_data[di..di + 4].copy_from_slice(&src_data[si..si + 4]);
                continue;
            }
            let inv = 255 - a;
            for c in 0..3 {
                dst_data[di + c] = (src_data[si + c] as u32
                    + dst_data[di + c] as u32 * inv / 255)
                    .min(255) as u8;
            }
            dst_data[di + 3] =
                (a + dst_data[di + 3] as u32 * inv / 255).min(255) as u8;
        }
    }
}

fn draw_checker(pixmap: &mut Pixmap, cell: f32) {    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let cell = (cell.max(4.0) as usize).max(1);
    let a = crate::render::theme::CHECKER_A;
    let b = crate::render::theme::CHECKER_B;
    let ca = [a.r, a.g, a.b, a.a];
    let cb = [b.r, b.g, b.b, b.a];
    let data = pixmap.data_mut();
    for y in 0..h {
        let row = y * w * 4;
        let y_cell = y / cell;
        let mut x = 0;
        while x < w {
            let x_cell = x / cell;
            let color = if (x_cell + y_cell) % 2 == 0 { ca } else { cb };
            let end = ((x_cell + 1) * cell).min(w);
            for xx in x..end {
                let idx = row + xx * 4;
                data[idx..idx + 4].copy_from_slice(&color);
            }
            x = end;
        }
    }
}

#[allow(dead_code)]
fn fill_pixmap(pixmap: &mut Pixmap, color: Rgba) {
    let mut paint = Paint::default();
    paint.set_color(color.to_color());
    pixmap.fill_rect(
        tiny_skia::Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32).unwrap(),
        &paint,
        Transform::identity(),
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::layer::Layer;
    use crate::model::Document;

    #[test]
    fn draws_scaled_image_at_offset() {
        let mut src = Pixmap::new(1, 1).unwrap();
        src.fill(Rgba::opaque(255, 0, 0).to_color());
        let mut doc = Document::new();
        doc.add(Layer::image(
            0,
            "red",
            Arc::new(src),
            Rect::new(10.0, 20.0, 30.0, 40.0),
        ));
        let mut text = TextEngine::new();
        let out = render_document(
            &doc,
            Rect::new(0.0, 0.0, 100.0, 100.0),
            1.0,
            Point::ZERO,
            100,
            100,
            Background::Transparent,
            &mut text,
            None,
            8.0,
            true,
        )
        .unwrap();
        let px = out.pixel(11, 21).unwrap();
        assert_eq!(px.red(), 255);
        assert_eq!(px.green(), 0);
        assert_eq!(px.alpha(), 255);
    }

    #[test]
    fn crop_trims_transparent() {
        let mut p = Pixmap::new(10, 10).unwrap();
        fill_rect(&mut p, Rect::new(3.0, 4.0, 4.0, 2.0), Rgba::opaque(1, 2, 3));
        let c = crop_transparent(p).unwrap();
        assert_eq!((c.width(), c.height()), (4, 2));
    }
}
