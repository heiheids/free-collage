mod doc;
mod export;
mod icons;
mod overlays;
mod theme;

use std::collections::HashMap;
use std::time::Instant;

pub use doc::{fit_view, page_screen_rect, render_page_view, Background};
pub use export::render_for_export;

use tiny_skia::Pixmap;
use crate::model::geometry::{Point, Rect};
use crate::model::ColorTarget;
use crate::platform::gdi::TextEngine;
use crate::render::icons::{
    draw_icon, draw_logo, fill_path, fill_rect, fill_round_rect, ring, round_rect_path,
    stroke_path, stroke_round_rect, Icon,
};
use crate::state::State;
use crate::tools::{Corner, Tool};
use crate::ui::{Hit, HitId, Layout};

pub struct Frame {
    pub pixmap: Pixmap,
    pub layout: Layout,
    pub hits: Vec<Hit>,
}

pub struct Renderer {
    pub text: TextEngine,
    panels: HashMap<(u32, u32, u32), Pixmap>,
    spare: Option<Pixmap>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            text: TextEngine::new(),
            panels: HashMap::new(),
            spare: None,
        }
    }

    pub fn recycle(&mut self, pixmap: Pixmap) {
        self.spare = Some(pixmap);
    }

    pub fn render(&mut self, state: &State, width: i32, height: i32, dpi: f32) -> Frame {
        let width = width.max(1) as f32;
        let height = height.max(1) as f32;
        let scale = (dpi / 96.0).clamp(0.75, 3.0);
        let layout = Layout::new(width, height, scale);
        let ui_boost = layout.ui_boost;
        let mut pixmap = self
            .spare
            .take()
            .filter(|p| p.width() == width as u32 && p.height() == height as u32)
            .unwrap_or_else(|| {
                Pixmap::new(width as u32, height as u32)
                    .unwrap_or_else(|| Pixmap::new(1, 1).expect("pixmap"))
            });
        pixmap.fill(theme::BG.to_color());
        let mut painter = Painter {
            state,
            text: &mut self.text,
            pixmap,
            hits: Vec::new(),
            layout,
            now: Instant::now(),
            ui_boost,
            panels: &mut self.panels,
        };
        painter.paint_base();
        painter.paint_popup();
        painter.paint_toast();
        painter.paint_modal();
        Frame {
            pixmap: painter.pixmap,
            layout: painter.layout,
            hits: painter.hits,
        }
    }
}

pub(crate) struct Painter<'a> {
    pub state: &'a State,
    pub text: &'a mut TextEngine,
    pub pixmap: Pixmap,
    pub hits: Vec<Hit>,
    pub layout: Layout,
    pub now: Instant,
    pub ui_boost: f32,
    panels: &'a mut HashMap<(u32, u32, u32), Pixmap>,
}

fn blit_opaque(dst: &mut Pixmap, src: &Pixmap, dx: i32, dy: i32) {
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let x0 = dx.max(0);
    let y0 = dy.max(0);
    let x1 = (dx + sw).min(dw);
    let y1 = (dy + sh).min(dh);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let src_x = (x0 - dx) as usize;
    let src_y = (y0 - dy) as usize;
    let copy_w = (x1 - x0) as usize;
    let dst_data = dst.data_mut();
    let src_data = src.data();
    for row in 0..(y1 - y0) as usize {
        let si = (((src_y + row) * sw as usize) + src_x) * 4;
        let di = ((((y0 as usize) + row) * dw as usize) + (x0 as usize)) * 4;
        dst_data[di..di + copy_w * 4].copy_from_slice(&src_data[si..si + copy_w * 4]);
    }
}

impl<'a> Painter<'a> {
    fn t(&self, key: &str) -> String {
        self.state.i18n.t(key).to_string()
    }

    fn push_hit(&mut self, id: HitId, rect: Rect) {
        self.hits.push(Hit::new(id, rect));
    }

    fn hover(&self, id: HitId) -> bool {
        self.state.hover == Some(id)
    }

    pub(crate) fn fill(&mut self, rect: Rect, color: crate::model::Rgba) {
        fill_rect(&mut self.pixmap, rect, color);
    }

    pub(crate) fn round(&mut self, rect: Rect, radius: f32, color: crate::model::Rgba) {
        fill_round_rect(&mut self.pixmap, rect, radius, color);
    }

    pub(crate) fn outline(
        &mut self,
        rect: Rect,
        radius: f32,
        color: crate::model::Rgba,
        width: f32,
    ) {
        stroke_round_rect(&mut self.pixmap, rect, radius, color, width);
    }

    fn panel(&mut self, rect: Rect, radius: f32) {
        let w = rect.w.round().max(1.0) as u32;
        let h = rect.h.round().max(1.0) as u32;
        let key = (w, h, radius.round().max(0.0) as u32);
        if !self.panels.contains_key(&key) {
            if self.panels.len() > 8 {
                self.panels.clear();
            }
            let Some(mut panel) = Pixmap::new(w, h) else {
                return;
            };
            panel.fill(theme::BG.to_color());
            fill_round_rect(
                &mut panel,
                Rect::new(0.0, 0.0, w as f32, h as f32),
                radius,
                theme::PANEL,
            );
            stroke_round_rect(
                &mut panel,
                Rect::new(0.0, 0.0, w as f32, h as f32),
                radius,
                theme::LINE,
                1.0,
            );
            self.panels.insert(key, panel);
        }
        let Some(panel) = self.panels.get(&key) else {
            return;
        };
        blit_opaque(
            &mut self.pixmap,
            panel,
            rect.x.round() as i32,
            rect.y.round() as i32,
        );
    }

    fn draw_raw(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: u32,
        color: crate::model::Rgba,
    ) {
        self.text
            .draw(&mut self.pixmap, x, y, text, size, weight, color, 1.0);
    }

    pub(crate) fn text_at(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: u32,
        color: crate::model::Rgba,
    ) {
        let size = size * self.ui_boost;
        self.draw_raw(x, y, text, size, weight, color);
    }

    fn text_in(
        &mut self,
        rect: Rect,
        text: &str,
        size: f32,
        weight: u32,
        color: crate::model::Rgba,
        align: Align,
    ) {
        let size = size * self.ui_boost;
        let (tw, th) = self.text.measure(text, size, weight);
        let x = match align {
            Align::Left => rect.x,
            Align::Center => rect.x + (rect.w - tw) * 0.5,
            Align::Right => rect.right() - tw,
        };
        let y = rect.y + (rect.h - th) * 0.5;
        self.draw_raw(x, y, text, size, weight, color);
    }

    fn text_key(
        &mut self,
        rect: Rect,
        key: &str,
        size: f32,
        weight: u32,
        color: crate::model::Rgba,
        align: Align,
    ) {
        let s = self.t(key);
        self.text_in(rect, &s, size, weight, color, align);
    }

    fn wrapped(
        &mut self,
        rect: Rect,
        text: &str,
        size: f32,
        weight: u32,
        color: crate::model::Rgba,
        align: Align,
    ) {
        let size = size * self.ui_boost;
        let lines = self.wrap(text, rect.w, size, weight);
        let (_, lh) = self.text.measure("Ag", size, weight);
        let lh = lh.max(size * 1.25);
        let total = lh * lines.len() as f32;
        let mut y = rect.y + (rect.h - total).max(0.0) * 0.5;
        for line in lines {
            let (tw, _) = self.text.measure(&line, size, weight);
            let x = match align {
                Align::Left => rect.x,
                Align::Center => rect.x + (rect.w - tw) * 0.5,
                Align::Right => rect.right() - tw,
            };
            self.draw_raw(x, y, &line, size, weight, color);
            y += lh;
        }
    }

    fn wrap(&mut self, text: &str, max_w: f32, size: f32, weight: u32) -> Vec<String> {
        let mut out = Vec::new();
        for para in text.split('\n') {
            if para.is_empty() {
                out.push(String::new());
                continue;
            }
            let tokens: Vec<String> = if para.contains(' ') {
                para.split(' ').map(|s| s.to_string()).collect()
            } else {
                para.chars().map(|c| c.to_string()).collect()
            };
            let joiner = if para.contains(' ') { " " } else { "" };
            let mut line = String::new();
            for token in tokens {
                let candidate = if line.is_empty() {
                    token.clone()
                } else {
                    format!("{line}{joiner}{token}")
                };
                let (w, _) = self.text.measure(&candidate, size, weight);
                if w > max_w && !line.is_empty() {
                    out.push(line);
                    line = token;
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                out.push(line);
            }
        }
        if out.is_empty() {
            out.push(String::new());
        }
        out
    }

    fn hit_tool(&mut self, tool: Tool, rect: Rect) {
        self.push_hit(HitId::Tool(tool), rect);
    }

    pub(crate) fn paint_base(&mut self) {
        self.ui_boost = self.layout.ui_boost;
        self.paint_header();
        self.paint_toolbar();
        self.ui_boost = 1.0;
        self.paint_canvas();
        self.ui_boost = self.layout.ui_boost;
        self.paint_bottom();
        self.ui_boost = 1.0;
        self.paint_selection();
    }

    fn paint_header(&mut self) {
        let l = self.layout.clone();
        self.fill(l.header, theme::WHITE);
        self.fill(Rect::new(0.0, l.header.bottom() - 1.0, l.width, 1.0), theme::LINE);
        draw_logo(&mut self.pixmap, l.logo);

        let title = self.t("app.title");
        self.text_at(l.title.x, l.title.y, &title, l.px(18.0), 600, theme::INK);

        let chip = l.canvas_chip;
        self.round(chip, l.px(8.0), theme::PANEL_ALT);
        self.outline(chip, l.px(8.0), theme::LINE, 1.0);
        let label = self.t("header.canvas");
        let value = if l.large {
            self.canvas_label()
        } else {
            self.canvas_label_short()
        };
        let top_size = l.px(11.5);
        let draw_size = top_size * self.ui_boost;
        let (lw, lh) = self.text.measure(&label, draw_size, 400);
        let (vw, vh) = self.text.measure(&value, draw_size, 600);
        let gap = l.px(8.0);
        let text_x = chip.x + l.px(12.0);
        self.text_at(text_x, chip.center().y - lh * 0.5, &label, top_size, 400, theme::MUTED);
        self.text_at(text_x + lw + gap, chip.center().y - vh * 0.5, &value, top_size, 600, theme::INK);
        draw_icon(
            &mut self.pixmap,
            Icon::ChevronDown,
            Rect::new(chip.right() - l.px(24.0), chip.center().y - l.px(7.0), l.px(14.0), l.px(14.0)),
            theme::MUTED,
            l.px(1.6),
        );
        self.push_hit(HitId::CanvasChip, chip);

        let shot = l.screenshot;
        if self.hover(HitId::Screenshot) {
            self.round(shot, l.px(8.0), theme::HOVER);
        } else {
            self.round(shot, l.px(8.0), theme::PANEL_ALT);
        }
        self.outline(shot, l.px(8.0), theme::LINE, 1.0);
        let shot_label = self.t("header.screenshot");
        self.text_in(shot, &shot_label, l.px(11.5), 500, theme::INK, Align::Center);
        self.push_hit(HitId::Screenshot, shot);

        let lang = l.lang;
        if self.hover(HitId::Language) {
            self.round(lang, l.px(8.0), theme::HOVER);
        }
        draw_icon(
            &mut self.pixmap,
            Icon::Globe,
            Rect::new(lang.x + l.px(9.0), lang.y + l.px(9.0), l.px(16.0), l.px(16.0)),
            theme::MUTED,
            l.px(1.5),
        );
        let name = self.state.i18n.current_name().to_string();
        self.text_in(
            Rect::new(lang.x + l.px(30.0), lang.y, l.px(64.0), lang.h),
            &name,
            l.px(11.5),
            500,
            theme::INK,
            Align::Left,
        );
        self.push_hit(HitId::Language, lang);

        let about = l.about;
        if self.hover(HitId::About) {
            self.round(about, l.px(8.0), theme::HOVER);
        }
        draw_icon(
            &mut self.pixmap,
            Icon::Info,
            Rect::new(about.x + l.px(8.0), about.y + l.px(8.0), l.px(18.0), l.px(18.0)),
            theme::MUTED,
            l.px(1.5),
        );
        self.push_hit(HitId::About, about);

        for (id, rect, icon, close) in [
            (HitId::Min, l.min, Icon::Minimize, false),
            (HitId::Max, l.max, if self.state.is_maximized { Icon::Restore } else { Icon::Maximize }, false),
            (HitId::Close, l.close, Icon::Close, true),
        ] {
            let hover = self.state.modal.is_none() && self.hover(id);
            if hover {
                self.fill(rect, if close { theme::DANGER_SOFT } else { theme::HOVER });
            }
            let color = if close && hover { theme::DANGER } else { theme::MUTED };
            draw_icon(
                &mut self.pixmap,
                icon,
                Rect::new(rect.center().x - l.px(7.0), rect.center().y - l.px(7.0), l.px(14.0), l.px(14.0)),
                color,
                l.px(1.5),
            );
            self.push_hit(id, rect);
        }
    }

    pub(crate) fn canvas_label(&self) -> String {
        let canvas = &self.state.doc.canvas;
        match canvas {
            crate::model::CanvasMode::Auto => self.state.i18n.t("canvas.auto").to_string(),
            crate::model::CanvasMode::Fixed {
                width_mm,
                height_mm,
                dpi,
            } => {
                let a4 = (width_mm - 210.0).abs() < 0.6 && (height_mm - 297.0).abs() < 0.6;
                let a4l = (width_mm - 297.0).abs() < 0.6 && (height_mm - 210.0).abs() < 0.6;
                let b5 = (width_mm - 176.0).abs() < 0.6 && (height_mm - 250.0).abs() < 0.6;
                let b5l = (width_mm - 250.0).abs() < 0.6 && (height_mm - 176.0).abs() < 0.6;
                let name = if a4 {
                    self.state.i18n.t("canvas.a4").to_string()
                } else if a4l {
                    self.state.i18n.t("canvas.a4_landscape").to_string()
                } else if b5 {
                    self.state.i18n.t("canvas.b5").to_string()
                } else if b5l {
                    self.state.i18n.t("canvas.b5_landscape").to_string()
                } else {
                    self.state.i18n.t("canvas.custom").to_string()
                };
                let short = name.split(' ').next().unwrap_or(&name);
                format!("{short} · {}×{}mm · {}dpi", trim_f(*width_mm), trim_f(*height_mm), trim_f(*dpi))
            }
        }
    }

    pub(crate) fn canvas_label_short(&self) -> String {
        let canvas = &self.state.doc.canvas;
        match canvas {
            crate::model::CanvasMode::Auto => self.state.i18n.t("canvas.auto").to_string(),
            crate::model::CanvasMode::Fixed {
                width_mm,
                height_mm,
                ..
            } => {
                let a4 = ((*width_mm - 210.0).abs() < 0.6 && (*height_mm - 297.0).abs() < 0.6)
                    || ((*width_mm - 297.0).abs() < 0.6 && (*height_mm - 210.0).abs() < 0.6);
                let b5 = ((*width_mm - 176.0).abs() < 0.6 && (*height_mm - 250.0).abs() < 0.6)
                    || ((*width_mm - 250.0).abs() < 0.6 && (*height_mm - 176.0).abs() < 0.6);
                if a4 {
                    "A4".into()
                } else if b5 {
                    "B5".into()
                } else {
                    self.state.i18n.t("canvas.custom").to_string()
                }
            }
        }
    }

    fn paint_toolbar(&mut self) {
        let l = self.layout.clone();
        self.panel(l.toolbar, l.px(13.0));
        let inner = l.toolbar.inflate(-l.px(8.0));
        let col_gap = l.px(8.0);
        let bw = (inner.w - col_gap) * 0.5;
        let div_h = inner.h - l.px(12.0);
        if div_h > 1.0 {
            let dx = (inner.x + bw + col_gap * 0.5).round();
            self.fill(
                Rect::new(dx, inner.y + l.px(6.0), 1.5, div_h),
                theme::LINE,
            );
        }

        for (tool, rect) in l.tools.iter() {
            let active = *tool == self.state.tool && tool.is_mode();
            let enabled = match tool {
                Tool::Undo => self.state.history.can_undo(),
                Tool::Delete => self.state.selected.is_some(),
                Tool::Save => self.state.page_rect().is_some(),
                Tool::Clear => !self.state.doc.layers.is_empty(),
                _ => true,
            };
            if active {
                self.round(*rect, l.px(8.0), theme::BLUE_SOFT);
            } else if self.hover(HitId::Tool(*tool)) && enabled {
                self.round(*rect, l.px(8.0), theme::HOVER);
            }
            let color = if !enabled {
                theme::MUTED.with_alpha(190)
            } else if active {
                theme::BLUE
            } else {
                theme::INK
            };
            let small = rect.h < l.px(48.0);
            let isize = if small { l.px(17.0) } else { l.px(24.0) };
            let fs = if small { l.px(8.5) } else { l.px(11.5) };
            let gap = l.px(2.5);
            let label_h = fs * self.ui_boost * 1.35;
            let total = isize + gap + label_h;
            let top = rect.y + ((rect.h - total) * 0.5).max(l.px(1.0));
            draw_icon(
                &mut self.pixmap,
                tool_icon(*tool),
                Rect::new(rect.center().x - isize * 0.5, top, isize, isize),
                color,
                l.px(1.7),
            );
            let label = self.t(tool.label_key());
            let label_rect = Rect::new(rect.x, top + isize + gap, rect.w, label_h + l.px(2.0));
            self.text_in(label_rect, &label, fs, 400, color, Align::Center);
            self.hit_tool(*tool, *rect);
        }
    }

    fn paint_canvas(&mut self) {
        let l = self.layout.clone();
        let page = self.state.page_rect();
        if page.is_none() {
            self.paint_transparent_workspace();
        }
        if let Some(page) = page {
            let screen = page_screen_rect(page, self.state.view.zoom, self.state.view.pan);
            if !screen.intersect(l.canvas).is_empty() {
                let background = if self.state.doc.canvas.is_auto() {
                    Background::Checker
                } else {
                    Background::White
                };
                if let Some((pix, origin)) = render_page_view(
                    &self.state.doc,
                    page,
                    self.state.view.zoom,
                    self.state.view.pan,
                    l.canvas,
                    background,
                    self.text,
                    self.state.text_edit.as_ref().and_then(|e| e.layer_id),
                    l.px(10.0),
                ) {
                    blit_opaque(
                        &mut self.pixmap,
                        &pix,
                        origin.x.round() as i32,
                        origin.y.round() as i32,
                    );
                }
                self.outline(screen, l.px(3.0), theme::LINE_STRONG, l.px(1.0));
            }
        }

        self.paint_preview();

        if self.state.mouse_in_canvas
            && matches!(self.state.tool, Tool::Brush | Tool::Mosaic | Tool::Eraser)
            && self.state.modal.is_none()
            && self.state.popup.is_none()
        {
            let r = (self.state.style.brush_size * self.state.view.zoom * 0.5).max(2.0);
            ring(&mut self.pixmap, self.state.mouse, r, theme::FAINT, l.px(1.0));
        }
        if self.state.text_edit.is_some() {
            let hint = self.t("hint.text_edit");
            let (tw, th) = self.text.measure(&hint, l.px(10.0), 400);
            let rect = Rect::new(
                l.canvas.center().x - tw * 0.5 - l.px(12.0),
                l.canvas.bottom() - th - l.px(18.0),
                tw + l.px(24.0),
                th + l.px(10.0),
            );
            self.round(rect, l.px(7.0), theme::TOAST.with_alpha(230));
            self.text_in(rect, &hint, l.px(10.0), 400, theme::WHITE, Align::Center);
        }
    }

    fn paint_transparent_workspace(&mut self) {
        let l = self.layout.clone();
        self.fill(l.canvas, theme::CHECKER_A);
        let cell = (18.0 * l.scale).max(8.0);
        let cols = (l.canvas.w / cell).ceil() as i32 + 1;
        let rows = (l.canvas.h / cell).ceil() as i32 + 1;
        for y in 0..rows {
            for x in 0..cols {
                if (x + y) % 2 == 0 {
                    continue;
                }
                let r = Rect::new(
                    l.canvas.x + x as f32 * cell,
                    l.canvas.y + y as f32 * cell,
                    cell,
                    cell,
                )
                .intersect(l.canvas);
                self.fill(r, theme::CHECKER_B);
            }
        }
    }

    fn paint_preview(&mut self) {
        let l = self.layout.clone();
        let z = self.state.view.zoom.max(0.02);
        let p = self.state.view.pan;
        let sx = |v: f32| v * z + p.x;
        let sy = |v: f32| v * z + p.y;
        match &self.state.drag {
            crate::tools::Drag::Stroke { points } => {
                if points.len() < 1 {
                    return;
                }
                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(sx(points[0].x), sy(points[0].y));
                for point in &points[1..] {
                    pb.line_to(sx(point.x), sy(point.y));
                }
                if let Some(path) = pb.finish() {
                    stroke_path(
                        &mut self.pixmap,
                        &path,
                        self.state.style.border,
                        (self.state.style.brush_size * z).max(0.6),
                    );
                }
            }
            crate::tools::Drag::ShapeCreate { kind, start, current } => {
                let rect = Rect::new(
                    sx(start.x.min(current.x)),
                    sy(start.y.min(current.y)),
                    (current.x - start.x).abs() * z,
                    (current.y - start.y).abs() * z,
                );
                let color = self.state.style.border;
                let width = (self.state.style.stroke_width * z).max(0.6);
                match kind {
                    crate::model::ShapeKind::Rect => {
                        if let Some(path) = round_rect_path(rect, 0.0) {
                            if let Some(fill) = self.state.style.fill {
                                fill_path(&mut self.pixmap, &path, fill);
                            }
                            stroke_path(&mut self.pixmap, &path, color, width);
                        }
                    }
                    crate::model::ShapeKind::Border => {
                        if let Some(path) = round_rect_path(rect, l.px(10.0)) {
                            stroke_path(&mut self.pixmap, &path, color, width);
                        }
                    }
                    crate::model::ShapeKind::Ellipse => {
                        if let Some(path) = oval_preview_path(rect) {
                            if let Some(fill) = self.state.style.fill {
                                fill_path(&mut self.pixmap, &path, fill);
                            }
                            stroke_path(&mut self.pixmap, &path, color, width);
                        }
                    }
                    crate::model::ShapeKind::Arrow => {
                        let a = crate::model::Point::new(sx(start.x), sy(start.y));
                        let b = crate::model::Point::new(sx(current.x), sy(current.y));
                        let mut pb = tiny_skia::PathBuilder::new();
                        pb.move_to(a.x, a.y);
                        pb.line_to(b.x, b.y);
                        if let Some(path) = pb.finish() {
                            stroke_path(&mut self.pixmap, &path, color, width);
                        }
                        let angle = (b.y - a.y).atan2(b.x - a.x);
                        let size = (width * 4.5).max(9.0);
                        let mut head = tiny_skia::PathBuilder::new();
                        head.move_to(b.x, b.y);
                        head.line_to(b.x - angle.cos() * size + angle.sin() * size * 0.55, b.y - angle.sin() * size - angle.cos() * size * 0.55);
                        head.move_to(b.x, b.y);
                        head.line_to(b.x - angle.cos() * size - angle.sin() * size * 0.55, b.y - angle.sin() * size + angle.cos() * size * 0.55);
                        if let Some(path) = head.finish() {
                            stroke_path(&mut self.pixmap, &path, color, width);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn primary_button(
        &mut self,
        rect: Rect,
        id: HitId,
        key: &str,
        icon: Option<Icon>,
    ) {
        let hover = self.hover(id);
        self.round(rect, self.layout.px(8.0), if hover { theme::BLUE_HOVER } else { theme::BLUE });
        let color = if hover { theme::BLUE } else { theme::WHITE };
        if hover {
            self.outline(rect, self.layout.px(8.0), theme::BLUE, 1.0);
        }
        let label = self.t(key);
        let (tw, _) = self.text.measure(&label, self.layout.px(12.0), 500);
        let icon_w = icon.map(|_| self.layout.px(20.0)).unwrap_or(0.0);
        let start = rect.center().x - (tw + icon_w + if icon.is_some() { self.layout.px(7.0) } else { 0.0 }) * 0.5;
        let mut x = start;
        if let Some(icon) = icon {
            draw_icon(
                &mut self.pixmap,
                icon,
                Rect::new(x, rect.center().y - self.layout.px(9.0), self.layout.px(18.0), self.layout.px(18.0)),
                color,
                self.layout.px(1.5),
            );
            x += icon_w + self.layout.px(7.0);
        }
        self.text_at(x, rect.center().y - self.layout.px(9.0), &label, self.layout.px(12.0), 500, color);
        self.push_hit(id, rect);
    }

    pub(crate) fn secondary_button(
        &mut self,
        rect: Rect,
        id: HitId,
        key: &str,
        icon: Option<Icon>,
    ) {
        let hover = self.hover(id);
        self.round(rect, self.layout.px(8.0), if hover { theme::HOVER } else { theme::PANEL });
        self.outline(rect, self.layout.px(8.0), if hover { theme::LINE_STRONG } else { theme::LINE }, 1.0);
        let color = theme::INK;
        let label = self.t(key);
        let (tw, _) = self.text.measure(&label, self.layout.px(12.0), 500);
        let icon_w = icon.map(|_| self.layout.px(20.0)).unwrap_or(0.0);
        let start = rect.center().x - (tw + icon_w + if icon.is_some() { self.layout.px(7.0) } else { 0.0 }) * 0.5;
        let mut x = start;
        if let Some(icon) = icon {
            draw_icon(
                &mut self.pixmap,
                icon,
                Rect::new(x, rect.center().y - self.layout.px(9.0), self.layout.px(18.0), self.layout.px(18.0)),
                color,
                self.layout.px(1.5),
            );
            x += icon_w + self.layout.px(7.0);
        }
        self.text_at(x, rect.center().y - self.layout.px(9.0), &label, self.layout.px(12.0), 500, color);
        self.push_hit(id, rect);
    }

    fn paint_bottom(&mut self) {
        let l = self.layout.clone();
        self.panel(l.bottom, l.px(14.0));

        let label_y = l.bottom.y + if l.large { l.px(8.0) } else { l.px(10.0) };
        let shape = self.t("settings.border_color");
        self.text_at(l.border_color.x, label_y, &shape, l.px(9.5), 400, theme::MUTED);
        let fill = self.t("settings.fill_color");
        self.text_at(l.fill_color.x, label_y, &fill, l.px(9.5), 400, theme::MUTED);
        let text = self.t("settings.text_color");
        self.text_at(l.text_color.x, label_y, &text, l.px(9.5), 400, theme::MUTED);
        let font = self.t("settings.font");
        self.text_at(l.font_minus.x, label_y, &font, l.px(9.5), 400, theme::MUTED);

        self.color_chip(l.border_color, HitId::BorderColor, Some(self.state.style.border));
        self.color_chip(l.fill_color, HitId::FillColor, self.state.style.fill);
        self.color_chip(l.text_color, HitId::TextColor, Some(self.state.style.text));

        self.stepper(
            l.font_minus,
            l.font_value,
            l.font_plus,
            HitId::FontMinus,
            HitId::FontPlus,
            format!("{:.0}", self.state.style.font_size),
        );

        let family_label = self.t("settings.font_family");
        self.text_at(l.font_family.x, label_y, &family_label, l.px(9.5), 400, theme::MUTED);
        self.round(l.font_family, l.px(7.0), theme::PANEL_ALT);
        self.outline(l.font_family, l.px(7.0), theme::LINE, 1.0);
        let family = crate::model::style::font_label(&self.state.style.font_family);
        self.text_in(
            Rect::new(
                l.font_family.x + l.px(9.0),
                l.font_family.y,
                l.font_family.w - l.px(30.0),
                l.font_family.h,
            ),
            family,
            l.px(10.5),
            500,
            theme::INK,
            Align::Left,
        );
        draw_icon(
            &mut self.pixmap,
            Icon::ChevronDown,
            Rect::new(
                l.font_family.right() - l.px(21.0),
                l.font_family.center().y - l.px(6.0),
                l.px(12.0),
                l.px(12.0),
            ),
            theme::MUTED,
            l.px(1.4),
        );
        self.push_hit(HitId::FontFamily, l.font_family);

        let ly2 = l.bottom.y + if l.large { l.px(92.0) } else { l.px(68.0) };
        let stroke = self.t("settings.stroke");
        self.text_at(l.stroke_minus.x, ly2, &stroke, l.px(9.5), 400, theme::MUTED);
        let brush = self.t("settings.brush");
        self.text_at(l.brush_minus.x, ly2, &brush, l.px(9.5), 400, theme::MUTED);
        let mosaic = self.t("settings.mosaic");
        self.text_at(l.mosaic_minus.x, ly2, &mosaic, l.px(9.5), 400, theme::MUTED);

        self.stepper(
            l.stroke_minus,
            l.stroke_value,
            l.stroke_plus,
            HitId::StrokeMinus,
            HitId::StrokePlus,
            trim_f(self.state.style.stroke_width),
        );
        self.stepper(
            l.brush_minus,
            l.brush_value,
            l.brush_plus,
            HitId::BrushMinus,
            HitId::BrushPlus,
            trim_f(self.state.style.brush_size),
        );
        self.stepper(
            l.mosaic_minus,
            l.mosaic_value,
            l.mosaic_plus,
            HitId::MosaicMinus,
            HitId::MosaicPlus,
            trim_f(self.state.style.mosaic_size),
        );

        let hint = if self.state.text_edit.is_some() {
            self.t("hint.text_edit")
        } else {
            self.t("hint.canvas")
        };
        let hint_rect = Rect::new(
            l.mosaic_plus.right() + l.px(24.0),
            l.bottom.y + l.px(90.0),
            (l.actual_size.x - l.mosaic_plus.right() - l.px(36.0)).max(0.0),
            l.px(20.0),
        );
        if !l.large && hint_rect.w > l.px(160.0) {
            self.text_in(hint_rect, &hint, l.px(10.0), 400, theme::FAINT, Align::Left);
        }

        if let Some(label) = self.actual_size_label() {
            self.text_in(l.actual_size, &label, l.px(10.0), 500, theme::MUTED, Align::Right);
        }

        self.stepper_icon(l.zoom_out, Icon::Minus, HitId::ZoomOut);
        self.text_in(
            l.zoom_value,
            &format!("{:.0}%", self.state.view.zoom * 100.0),
            l.px(11.0),
            500,
            theme::INK,
            Align::Center,
        );
        self.stepper_icon(l.zoom_in, Icon::Plus, HitId::ZoomIn);
        let fit = self.t("settings.fit");
        self.round(l.zoom_fit, l.px(7.0), if self.hover(HitId::ZoomFit) { theme::HOVER } else { theme::PANEL_ALT });
        self.text_in(l.zoom_fit, &fit, l.px(10.5), 500, theme::INK, Align::Center);
        self.push_hit(HitId::ZoomFit, l.zoom_fit);
    }

    fn actual_size_label(&self) -> Option<String> {
        let (w, h) = if let Some(layer) = self.state.selected_layer() {
            (layer.rect.w, layer.rect.h)
        } else {
            let (w, h) = self.state.page_pixels()?;
            (w as f32, h as f32)
        };
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let w = format!("{:.0}", w.max(1.0));
        let h = format!("{:.0}", h.max(1.0));
        Some(self.state.i18n.fmt("bottom.actual_size", &[&w, &h]))
    }

    fn color_chip(&mut self, rect: Rect, id: HitId, color: Option<crate::model::Rgba>) {
        let l = self.layout.clone();
        self.round(rect, l.px(7.0), theme::PANEL_ALT);
        self.outline(rect, l.px(7.0), if self.hover(id) { theme::LINE_STRONG } else { theme::LINE }, 1.0);
        let sw = Rect::new(rect.x + l.px(8.0), rect.center().y - l.px(9.0), l.px(18.0), l.px(18.0));
        if let Some(color) = color {
            self.round(sw, l.px(4.0), theme::WHITE);
            self.outline(sw, l.px(4.0), theme::LINE_STRONG, 1.0);
            self.round(sw.inflate(-l.px(2.0)), l.px(3.0), color);
        } else {
            self.round(sw, l.px(4.0), theme::CHECKER_B);
            self.round(Rect::new(sw.x, sw.y, sw.w * 0.5, sw.h * 0.5), 0.0, theme::CHECKER_A);
            self.outline(sw, l.px(4.0), theme::LINE_STRONG, 1.0);
        }
        let label = if let Some(c) = color {
            c.to_hex()
        } else {
            self.t("settings.none")
        };
        self.text_in(
            Rect::new(sw.right() + l.px(9.0), rect.y, rect.w - l.px(52.0), rect.h),
            &label,
            l.px(11.0),
            500,
            theme::INK,
            Align::Left,
        );
        draw_icon(
            &mut self.pixmap,
            Icon::ChevronDown,
            Rect::new(rect.right() - l.px(22.0), rect.center().y - l.px(7.0), l.px(14.0), l.px(14.0)),
            theme::MUTED,
            l.px(1.5),
        );
        self.push_hit(id, rect);
    }

    fn stepper(
        &mut self,
        minus: Rect,
        value: Rect,
        plus: Rect,
        id_minus: HitId,
        id_plus: HitId,
        text: String,
    ) {
        self.stepper_icon(minus, Icon::Minus, id_minus);
        self.text_in(value, &text, self.layout.px(11.5), 500, theme::INK, Align::Center);
        self.stepper_icon(plus, Icon::Plus, id_plus);
        self.outline(
            Rect::new(minus.x, minus.y, plus.right() - minus.x, minus.h),
            self.layout.px(7.0),
            theme::LINE,
            1.0,
        );
    }

    fn stepper_icon(&mut self, rect: Rect, icon: Icon, id: HitId) {
        let l = self.layout.clone();
        if self.hover(id) {
            self.round(rect, l.px(7.0), theme::HOVER);
        }
        draw_icon(
            &mut self.pixmap,
            icon,
            Rect::new(rect.center().x - l.px(7.0), rect.center().y - l.px(7.0), l.px(14.0), l.px(14.0)),
            theme::MUTED,
            l.px(1.5),
        );
        self.push_hit(id, rect);
    }

    fn paint_selection(&mut self) {
        let l = self.layout.clone();
        if self.state.tool == Tool::Mosaic
            && self
                .state
                .selected_layer()
                .map(|layer| matches!(layer.kind, crate::model::LayerKind::Mosaic { .. }))
                .unwrap_or(false)
        {
            return;
        }
        let Some(sel) = self.state.selected_rect() else {
            return;
        };
        if self.state.text_edit.as_ref().and_then(|e| e.layer_id) == self.state.selected {
            return;
        }
        let screen = page_screen_rect(sel, self.state.view.zoom, self.state.view.pan);
        if screen.intersect(l.canvas).is_empty() {
            return;
        }
        let blue = theme::BLUE;
        let w = l.px(1.5);
        stroke_round_rect(&mut self.pixmap, screen, l.px(1.0), blue.with_alpha(120), w);
        for (_, h) in handle_rects(screen, &l) {
            if h.intersect(l.canvas).is_empty() {
                continue;
            }
            fill_round_rect(&mut self.pixmap, h, l.px(2.0), theme::WHITE);
            stroke_round_rect(&mut self.pixmap, h, l.px(2.0), blue, l.px(1.5));
        }
    }

    pub(crate) fn paint_toast(&mut self) {
        let Some(toast) = &self.state.toast else {
            return;
        };
        if self.now >= toast.until {
            return;
        }
        let l = self.layout.clone();
        let size = l.px(11.5);
        let text = toast.text.clone();
        let (tw, th) = self.text.measure(&text, size, 400);
        let max_w = (l.width - l.px(80.0)).max(l.px(200.0));
        let tw = tw.min(max_w);
        let rect = Rect::new(
            l.width * 0.5 - tw * 0.5 - l.px(18.0),
            l.bottom.y - th - l.px(30.0),
            tw + l.px(36.0),
            th + l.px(14.0),
        );
        self.round(rect, l.px(9.0), theme::TOAST);
        self.text_in(rect, &text, size, 400, theme::WHITE, Align::Center);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Center,
    Right,
}

pub fn handle_rects(rect: Rect, l: &Layout) -> [(Corner, Rect); 4] {
    let size = l.px(11.0);
    let half = size * 0.5;
    let mk = |p: Point| Rect::new(p.x - half, p.y - half, size, size);
    [
        (Corner::TopLeft, mk(Point::new(rect.x, rect.y))),
        (Corner::TopRight, mk(Point::new(rect.right(), rect.y))),
        (Corner::BottomLeft, mk(Point::new(rect.x, rect.bottom()))),
        (Corner::BottomRight, mk(Point::new(rect.right(), rect.bottom()))),
    ]
}

fn tool_icon(tool: Tool) -> Icon {
    match tool {
        Tool::Select => Icon::Select,
        Tool::Open => Icon::Open,
        Tool::Paste => Icon::Paste,
        Tool::Delete => Icon::Delete,
        Tool::Clear => Icon::Clear,
        Tool::Undo => Icon::Undo,
        Tool::Save => Icon::Save,
        Tool::Text => Icon::Text,
        Tool::Brush => Icon::Brush,
        Tool::Mosaic => Icon::Mosaic,
        Tool::Eraser => Icon::Eraser,
        Tool::Arrow => Icon::Arrow,
        Tool::Border => Icon::Border,
        Tool::Rect => Icon::Rect,
        Tool::Ellipse => Icon::Ellipse,
    }
}

fn trim_f(v: f32) -> String {
    if (v - v.round()).abs() < 0.01 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.1}")
    }
}

fn oval_preview_path(r: Rect) -> Option<tiny_skia::Path> {
    let rx = r.w * 0.5;
    let ry = r.h * 0.5;
    let cx = r.x + rx;
    let cy = r.y + ry;
    let k = 0.552_284_8;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx + rx, cy);
    pb.cubic_to(cx + rx, cy + ry * k, cx + rx * k, cy + ry, cx, cy + ry);
    pb.cubic_to(cx - rx * k, cy + ry, cx - rx, cy + ry * k, cx - rx, cy);
    pb.cubic_to(cx - rx, cy - ry * k, cx - rx * k, cy - ry, cx, cy - ry);
    pb.cubic_to(cx + rx * k, cy - ry, cx + rx, cy - ry * k, cx + rx, cy);
    pb.close();
    pb.finish()
}

pub(crate) fn color_target_label(state: &State, target: ColorTarget) -> String {
    match target {
        ColorTarget::Border => state.i18n.t("settings.border_color").to_string(),
        ColorTarget::Fill => state.i18n.t("settings.fill_color").to_string(),
        ColorTarget::Text => state.i18n.t("settings.text_color").to_string(),
        ColorTarget::TextOutline => state.i18n.t("settings.text_outline").to_string(),
    }
}
