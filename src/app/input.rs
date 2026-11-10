use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_0, VK_1, VK_A, VK_ADD, VK_B, VK_BACK, VK_DELETE, VK_E, VK_ESCAPE, VK_F, VK_F12, VK_M,
    VK_O, VK_OEM_MINUS, VK_OEM_PLUS, VK_R, VK_RETURN, VK_S, VK_SPACE, VK_SUBTRACT, VK_T, VK_TAB,
    VK_V, VK_Z,
};

use crate::model::layer::{Layer, LayerKind, ShapeKind};
use crate::model::{Point, Rect};
use crate::state::{Modal, Popup};
use crate::tools::{Corner, Cursor, Drag, Tool};
use crate::ui::HitId;

use super::App;

const VK_CONTROL: i32 = 0x11;
const VK_SHIFT: i32 = 0x10;

impl App {    fn hit_at(&self, p: Point) -> Option<HitId> {
        self.hits
            .iter()
            .rev()
            .find(|h| h.rect.contains(p))
            .map(|h| h.id)
    }

    fn popup_hit(id: HitId) -> bool {
        matches!(
            id,
            HitId::Swatch(_)
                | HitId::ColorNone
                | HitId::ColorCustom
                | HitId::CanvasAuto
                | HitId::CanvasA4
                | HitId::CanvasA4Landscape
                | HitId::CanvasB5
                | HitId::CanvasB5Landscape
                | HitId::CanvasCustom
                | HitId::CanvasField(_)
                | HitId::CanvasApply
                | HitId::LangAuto
                | HitId::LangItem(_)
                | HitId::FontChoice(_)
                | HitId::TextOutlineColor
                | HitId::LayerUp
                | HitId::LayerDown
        )
    }

    fn modal_hit(id: HitId) -> bool {
        matches!(
            id,
            HitId::SaveFormat(_)
                | HitId::SaveQuality
                | HitId::SaveTransparent
                | HitId::SaveExport
                | HitId::SaveCancel
                | HitId::CloseSave
                | HitId::CloseDiscard
                | HitId::CloseCancel
                | HitId::AboutClose
                | HitId::Homepage
        )
    }

    fn popup_anchor(&self) -> Option<HitId> {
        match self.state.popup {
            Some(Popup::Color { target, .. }) => Some(match target {
                crate::model::ColorTarget::Border => HitId::BorderColor,
                crate::model::ColorTarget::Fill => HitId::FillColor,
                crate::model::ColorTarget::Text => HitId::TextColor,
                crate::model::ColorTarget::TextOutline => HitId::FontFamily,
            }),
            Some(Popup::Canvas { .. }) => Some(HitId::CanvasChip),
            Some(Popup::Language { .. }) => Some(HitId::Language),
            Some(Popup::Font { .. }) => Some(HitId::FontFamily),
            Some(Popup::LayerMenu { .. }) => None,
            None => None,
        }
    }

    pub fn right_down(&mut self, x: f32, y: f32) {
        if self.state.modal.is_some() {
            return;
        }
        if !self.layout.canvas.contains(Point::new(x, y)) {
            self.state.close_popup();
            self.invalidate();
            return;
        }
        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }
        let zoom = self.state.view.zoom.max(0.01);
        let doc = Point::new(
            (x - self.state.view.pan.x) / zoom,
            (y - self.state.view.pan.y) / zoom,
        );
        if let Some(id) = self.state.doc.hit_test(doc, 6.0 / zoom) {
            self.open_layer_menu(id, Rect::new(x, y, 1.0, 1.0));
        } else {
            self.state.close_popup();
            self.invalidate();
        }
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) {
        self.state.mouse = Point::new(x, y);
        let old_in_canvas = self.state.mouse_in_canvas;
        let old_hover = self.state.hover;
        self.state.mouse_in_canvas = self.layout.canvas.contains(self.state.mouse);
        if self.state.slider_drag && self.state.modal == Some(Modal::Save) {
            self.set_quality_from_x(x);
        }
        let dragging = !matches!(self.state.drag, Drag::None);
        if dragging && self.mouse_down {
            self.drag_update();
        } else {
            self.state.hover = self.hit_at(self.state.mouse);
            self.update_cursor();
        }
        let ring = self.state.mouse_in_canvas
            && matches!(self.state.tool, Tool::Brush | Tool::Mosaic | Tool::Eraser);
        let visual_change = dragging
            || self.state.slider_drag
            || self.state.hover != old_hover
            || self.state.mouse_in_canvas != old_in_canvas
            || ring;
        if visual_change {
            self.invalidate();
        }
    }

    pub fn mouse_leave(&mut self) {
        self.state.hover = None;
        self.state.mouse_in_canvas = false;
        self.state.cursor = Cursor::Arrow;
        self.invalidate();
    }

    pub fn mouse_down(&mut self, x: f32, y: f32, button: u8) {
        self.state.mouse = Point::new(x, y);
        let p = self.state.mouse;
        if button == 2 {
            if self.layout.canvas.contains(p) {
                if self.state.text_edit.is_some() {
                    self.finish_text_edit(true);
                }
                self.state.drag = Drag::Pan {
                    start_mouse: p,
                    start_pan: self.state.view.pan,
                };
                self.mouse_down = true;
            }
            return;
        }

        if self.state.modal.is_some() {
            if let Some(id) = self.hit_at(p) {
                if Self::modal_hit(id) {
                    if id == HitId::SaveQuality {
                        self.state.slider_drag = true;
                        self.set_quality_from_x(x);
                    }
                    self.handle_hit(id);
                }
            }
            return;
        }

        if self.state.popup.is_some() {
            if let Some(id) = self.hit_at(p) {
                if Self::popup_hit(id) {
                    self.handle_hit(id);
                    return;
                }
                if Some(id) == self.popup_anchor() {
                    self.handle_hit(id);
                    return;
                }
            }
            self.state.close_popup();
            self.invalidate();
            return;
        }

        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }

        if let Some(id) = self.hit_at(p) {
            self.handle_hit(id);
            return;
        }

        if !self.layout.canvas.contains(p) {
            self.state.selected = None;
            self.invalidate();
            return;
        }

        if self.state.space_down {
            self.state.drag = Drag::Pan {
                start_mouse: p,
                start_pan: self.state.view.pan,
            };
            self.mouse_down = true;
            return;
        }

        self.mouse_down = true;
        self.canvas_down(p);
        self.invalidate();
    }

    fn canvas_down(&mut self, screen: Point) {
        let zoom = self.state.view.zoom.max(0.01);
        let doc = Point::new(
            (screen.x - self.state.view.pan.x) / zoom,
            (screen.y - self.state.view.pan.y) / zoom,
        );
        let tol = 6.0 / zoom;

        match self.state.tool {
            Tool::Select => {
                if let Some(sel) = self.state.selected_rect() {
                    let rect = crate::render::page_screen_rect(sel, zoom, self.state.view.pan);
                    for (corner, handle) in crate::render::handle_rects(rect, &self.layout) {
                        if handle.contains(screen) {
                            if let Some(layer) = self.state.selected_layer().cloned() {
                                self.state.drag = Drag::Scale {
                                    id: layer.id,
                                    corner,
                                    start_mouse: doc,
                                    start_layer: layer,
                                    pushed: false,
                                };
                                return;
                            }
                        }
                    }
                }
                if let Some(id) = self.state.doc.hit_test(doc, tol) {
                    self.state.selected = Some(id);
                    if let Some(layer) = self.state.doc.layer(id).cloned() {
                        self.state.drag = Drag::Move {
                            id,
                            start_mouse: doc,
                            start_layer: layer,
                            pushed: false,
                        };
                    }
                } else {
                    self.state.selected = None;
                    self.state.drag = Drag::Pan {
                        start_mouse: screen,
                        start_pan: self.state.view.pan,
                    };
                }
            }
            Tool::Text => {
                if let Some(id) = self.state.doc.hit_test(doc, tol) {
                    if self
                        .state
                        .doc
                        .layer(id)
                        .map(|l| l.is_text())
                        .unwrap_or(false)
                    {
                        self.begin_edit_text(id);
                        return;
                    }
                }
                self.begin_new_text(doc);
            }
            Tool::Brush => {
                self.state.drag = Drag::Stroke {
                    points: vec![doc],
                };
            }
            Tool::Mosaic => {
                self.state.drag = Drag::Mosaic {
                    id: None,
                    last: doc,
                    pushed: false,
                };
                self.mosaic_at(doc, tol);
            }
            Tool::Eraser => {
                self.state.drag = Drag::Erase {
                    last: doc,
                    pushed: false,
                };
                self.erase_at(doc, tol);
            }
            Tool::Arrow => {
                self.state.drag = Drag::ShapeCreate {
                    kind: ShapeKind::Arrow,
                    start: doc,
                    current: doc,
                };
            }
            Tool::Border => {
                self.state.drag = Drag::ShapeCreate {
                    kind: ShapeKind::Border,
                    start: doc,
                    current: doc,
                };
            }
            Tool::Rect => {
                self.state.drag = Drag::ShapeCreate {
                    kind: ShapeKind::Rect,
                    start: doc,
                    current: doc,
                };
            }
            Tool::Ellipse => {
                self.state.drag = Drag::ShapeCreate {
                    kind: ShapeKind::Ellipse,
                    start: doc,
                    current: doc,
                };
            }
            _ => {}
        }
    }

    fn drag_update(&mut self) {
        let zoom = self.state.view.zoom.max(0.01);
        let screen = self.state.mouse;
        let doc = Point::new(
            (screen.x - self.state.view.pan.x) / zoom,
            (screen.y - self.state.view.pan.y) / zoom,
        );
        match self.state.drag.clone() {
            Drag::Pan { start_mouse, start_pan } => {
                self.state.view.pan = Point::new(
                    start_pan.x + screen.x - start_mouse.x,
                    start_pan.y + screen.y - start_mouse.y,
                );
                self.state.view.fitted = false;
                self.state.clamp_pan(self.layout.canvas);
            }
            Drag::Move {
                id,
                start_mouse,
                start_layer,
                pushed,
            } => {
                let dx = doc.x - start_mouse.x;
                let dy = doc.y - start_mouse.y;
                if dx.abs() + dy.abs() > 0.2 {
                    if !pushed {
                        self.state.push_history();
                        if let Drag::Move { pushed, .. } = &mut self.state.drag {
                            *pushed = true;
                        }
                    }
                    let mut layer = start_layer;
                    layer.move_by(dx, dy);
                    if let Some(target) = self.state.doc.layer_mut(id) {
                        *target = layer;
                    }
                }
            }
            Drag::Scale {
                id,
                corner,
                start_mouse: _,
                start_layer,
                pushed,
            } => {
                let start_rect = start_layer.rect;
                let anchor = corner.anchor(start_rect);
                let ref_x = if matches!(corner, Corner::TopLeft | Corner::BottomLeft) {
                    start_rect.x
                } else {
                    start_rect.right()
                };
                let ref_y = if matches!(corner, Corner::TopLeft | Corner::TopRight) {
                    start_rect.y
                } else {
                    start_rect.bottom()
                };
                let denom_x = ref_x - anchor.x;
                let denom_y = ref_y - anchor.y;
                if denom_x.abs() < 0.01 || denom_y.abs() < 0.01 {
                    return;
                }
                let sx = (doc.x - anchor.x) / denom_x;
                let sy = (doc.y - anchor.y) / denom_y;
                let uniform = start_layer.is_image()
                    || start_layer.is_text()
                    || matches!(start_layer.kind, LayerKind::Mosaic { .. });
                let (sx, sy) = if uniform {
                    let s = if (sx - 1.0).abs() > (sy - 1.0).abs() { sx } else { sy };
                    (s.max(0.02), s.max(0.02))
                } else {
                    (sx.max(0.02), sy.max(0.02))
                };
                if !pushed {
                    self.state.push_history();
                    if let Drag::Scale { pushed, .. } = &mut self.state.drag {
                        *pushed = true;
                    }
                }
                let mut layer = start_layer;
                layer.scale_from(anchor, sx, sy);
                if let Some(target) = self.state.doc.layer_mut(id) {
                    *target = layer;
                }
            }
            Drag::ShapeCreate { current, .. } => {
                if let Drag::ShapeCreate { current: c, .. } = &mut self.state.drag {
                    *c = doc;
                }
            }
            Drag::Stroke { .. } => {
                if let Drag::Stroke { points } = &mut self.state.drag {
                    if points.last().map(|p| p.distance(doc)).unwrap_or(1.0) > 1.0 / zoom {
                        points.push(doc);
                    }
                }
            }
            Drag::Mosaic { .. } => {
                self.mosaic_segment(doc, 6.0 / zoom);
            }
            Drag::Erase { .. } => {
                self.erase_segment(doc, 6.0 / zoom);
            }
            Drag::None => {}
        }
    }

    pub fn mouse_up(&mut self) {
        let drag = std::mem::replace(&mut self.state.drag, Drag::None);
        match drag {
            Drag::Stroke { mut points } => {
                if points.is_empty() {
                } else {
                    if points.len() == 1 {
                        let p = points[0];
                        points.push(Point::new(p.x + 0.1, p.y + 0.1));
                    }
                    self.state.push_history();
                    let layer = Layer::stroke(
                        0,
                        points,
                        self.state.style.border,
                        self.state.style.brush_size,
                    );
                    let id = self.state.doc.add(layer);
                    self.state.selected = Some(id);
                }
            }
            Drag::ShapeCreate { kind, start, current } => {
                let rect = Rect::from_points(start, current);
                if rect.w > 2.0 || rect.h > 2.0 {
                    self.state.push_history();
                    let fill = match kind {
                        ShapeKind::Arrow | ShapeKind::Border => None,
                        _ => self.state.style.fill,
                    };
                    let layer = Layer::shape(
                        0,
                        kind,
                        start,
                        current,
                        self.state.style.border,
                        fill,
                        self.state.style.stroke_width,
                        10.0,
                    );
                    let id = self.state.doc.add(layer);
                    self.state.selected = Some(id);
                }
            }
            Drag::Mosaic { id, .. } => {
                if let Some(id) = id {
                    let empty = self
                        .state
                        .doc
                        .layer(id)
                        .map(|l| match &l.kind {
                            LayerKind::Mosaic { cells, .. } => cells.is_empty(),
                            _ => true,
                        })
                        .unwrap_or(true);
                    if empty {
                        self.state.doc.remove(id);
                    } else {
                        self.state.selected = Some(id);
                    }
                }
            }
            _ => {}
        }
        self.mouse_down = false;
        self.state.slider_drag = false;
        self.invalidate();
    }

    pub fn mouse_double(&mut self, x: f32, y: f32) {
        self.state.mouse = Point::new(x, y);
        if self.state.modal.is_some() || self.state.popup.is_some() {
            return;
        }
        if !self.layout.canvas.contains(self.state.mouse) {
            return;
        }
        let zoom = self.state.view.zoom.max(0.01);
        let doc = Point::new(
            (x - self.state.view.pan.x) / zoom,
            (y - self.state.view.pan.y) / zoom,
        );
        if let Some(id) = self.state.doc.hit_test(doc, 6.0 / zoom) {
            if self.state.doc.layer(id).map(|l| l.is_text()).unwrap_or(false) {
                self.state.selected = Some(id);
                self.begin_edit_text(id);
            }
        } else if self.state.tool == Tool::Select {
            self.fit_current();
        }
    }

    pub fn wheel(&mut self, x: f32, y: f32, delta: i32) {
        if !self.layout.canvas.contains(Point::new(x, y)) || self.state.modal.is_some() {
            return;
        }
        let factor = 1.15f32.powf(delta as f32 / 120.0);
        self.zoom_to(self.state.view.zoom * factor, Point::new(x, y));
    }

    pub fn key_up(&mut self, vk: usize) {
        if vk == VK_SPACE as usize {
            self.state.space_down = false;
            self.update_cursor();
            self.invalidate();
        }
    }

    pub fn key_down(&mut self, vk: usize, ctrl: bool, shift: bool) -> bool {
        if self.state.popup.is_some() {
            if let Some(Popup::Canvas { custom: true, .. }) = &self.state.popup {
                match vk {
                    x if x == VK_BACK as usize => {
                        self.state.canvas_form.active_text_mut().pop();
                        self.state.canvas_form.invalid = false;
                        self.invalidate();
                        return true;
                    }
                    x if x == VK_TAB as usize => {
                        self.state.canvas_form.focus_next(shift);
                        self.invalidate();
                        return true;
                    }
                    x if x == VK_RETURN as usize => {
                        self.apply_custom_canvas();
                        return true;
                    }
                    _ => {}
                }
            }
        }

        if vk == VK_F12 as usize {
            self.dump_state();
            return true;
        }

        if vk == VK_ESCAPE as usize {
            if self.state.modal.is_some() {
                self.state.modal = None;
            } else if self.state.popup.is_some() {
                self.state.close_popup();
            } else if !matches!(self.state.drag, Drag::None) {
                self.state.drag = Drag::None;
                self.mouse_down = false;
            } else {
                self.state.selected = None;
            }
            self.invalidate();
            return true;
        }

        if self.state.modal.is_some() || self.state.popup.is_some() {
            return false;
        }

        if vk == VK_SPACE as usize && !ctrl {
            self.state.space_down = true;
            self.state.cursor = Cursor::Hand;
            return true;
        }

        if ctrl {
            match vk {
                x if x == VK_Z as usize => {
                    self.undo();
                    return true;
                }
                x if x == VK_O as usize => {
                    self.open_dialog();
                    return true;
                }
                x if x == VK_V as usize => {
                    self.paste_image();
                    return true;
                }
                x if x == VK_S as usize => {
                    self.open_save_modal();
                    return true;
                }
                x if x == VK_0 as usize => {
                    self.state.view.fitted = false;
                    self.fit_current();
                    return true;
                }
                x if x == VK_1 as usize => {
                    self.zoom_to(1.0, self.layout.canvas.center());
                    return true;
                }
                _ => {}
            }
            return false;
        }

        match vk {
            x if x == VK_DELETE as usize => {
                self.delete_selected();
                true
            }
            x if x == VK_TAB as usize => {
                self.state.selected = self
                    .state
                    .doc
                    .cycle_selection(self.state.selected, shift);
                self.invalidate();
                true
            }
            x if x == VK_V as usize => {
                self.set_tool(Tool::Select);
                true
            }
            x if x == VK_T as usize => {
                self.set_tool(Tool::Text);
                true
            }
            x if x == VK_B as usize => {
                self.set_tool(Tool::Brush);
                true
            }
            x if x == VK_M as usize => {
                self.set_tool(Tool::Mosaic);
                true
            }
            x if x == VK_E as usize => {
                self.set_tool(Tool::Eraser);
                true
            }
            x if x == VK_A as usize => {
                self.set_tool(Tool::Arrow);
                true
            }
            x if x == VK_R as usize => {
                self.set_tool(Tool::Rect);
                true
            }
            x if x == VK_O as usize => {
                self.set_tool(Tool::Ellipse);
                true
            }
            x if x == VK_F as usize => {
                self.set_tool(Tool::Border);
                true
            }
            x if x == VK_OEM_PLUS as usize || x == VK_ADD as usize => {
                self.zoom_step(1.2);
                true
            }
            x if x == VK_OEM_MINUS as usize || x == VK_SUBTRACT as usize => {
                self.zoom_step(1.0 / 1.2);
                true
            }
            _ => false,
        }
    }

    fn set_tool(&mut self, tool: Tool) {
        self.state.tool = tool;
        self.state.close_popup();
        self.update_cursor();
        self.invalidate();
    }

    pub fn char_input(&mut self, ch: char) -> bool {
        if let Some(Popup::Canvas { custom: true, .. }) = &self.state.popup {
            if ch.is_ascii_digit() || ch == '.' {
                self.state.canvas_form.active_text_mut().push(ch);
                self.state.canvas_form.invalid = false;
                self.invalidate();
                return true;
            }
        }
        false
    }

    pub fn capture_lost(&mut self) {
        if self.mouse_down {
            self.mouse_up();
        }
    }

    fn update_cursor(&mut self) {
        if self.state.space_down {
            self.state.cursor = Cursor::Hand;
            return;
        }
        if self.state.modal.is_some() || self.state.popup.is_some() {
            self.state.cursor = Cursor::Arrow;
            return;
        }
        if !self.layout.canvas.contains(self.state.mouse) {
            self.state.cursor = Cursor::Arrow;
            return;
        }
        if self.state.tool == Tool::Select {
            if let Some(sel) = self.state.selected_rect() {
                let zoom = self.state.view.zoom.max(0.01);
                let rect = crate::render::page_screen_rect(sel, zoom, self.state.view.pan);
                for (corner, handle) in crate::render::handle_rects(rect, &self.layout) {
                    if handle.contains(self.state.mouse) {
                        self.state.cursor = match corner {
                            Corner::TopLeft | Corner::BottomRight => Cursor::SizeNWSE,
                            Corner::TopRight | Corner::BottomLeft => Cursor::SizeNESW,
                        };
                        return;
                    }
                }
            }
            if self
                .state
                .doc
                .hit_test(self.doc_point(self.state.mouse), 6.0 / self.state.view.zoom)
                .is_some()
            {
                self.state.cursor = Cursor::Move;
            } else {
                self.state.cursor = Cursor::Arrow;
            }
            return;
        }
        self.state.cursor = match self.state.tool {
            Tool::Text => Cursor::IBeam,
            Tool::Brush | Tool::Mosaic | Tool::Eraser | Tool::Arrow | Tool::Border | Tool::Rect | Tool::Ellipse => Cursor::Crosshair,
            _ => Cursor::Arrow,
        };
    }

    fn doc_point(&self, screen: Point) -> Point {
        let zoom = self.state.view.zoom.max(0.01);
        Point::new(
            (screen.x - self.state.view.pan.x) / zoom,
            (screen.y - self.state.view.pan.y) / zoom,
        )
    }

    fn mosaic_segment(&mut self, to: Point, tol: f32) {
        let last = match &self.state.drag {
            Drag::Mosaic { last, .. } => *last,
            _ => return,
        };
        let cell = self.state.style.mosaic_size.max(3.0);
        let dist = last.distance(to);
        let steps = (dist / (cell * 0.55).max(1.0)).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let p = last.lerp(to, i as f32 / steps as f32);
            self.mosaic_at(p, tol);
        }
        if let Drag::Mosaic { last, .. } = &mut self.state.drag {
            *last = to;
        }
    }

    fn mosaic_at(&mut self, p: Point, _tol: f32) {
        let cell = self.state.style.mosaic_size.max(3.0);
        let mut id = match &self.state.drag {
            Drag::Mosaic { id, .. } => *id,
            _ => None,
        };
        if id.is_none() {
            let pushed = matches!(&self.state.drag, Drag::Mosaic { pushed: true, .. });
            if !pushed {
                self.state.push_history();
                if let Drag::Mosaic { pushed, .. } = &mut self.state.drag {
                    *pushed = true;
                }
            }
            let new_id = self.state.doc.add(Layer::mosaic(0, cell));
            if let Drag::Mosaic { id: slot, .. } = &mut self.state.drag {
                *slot = Some(new_id);
            }
            id = Some(new_id);
            self.state.selected = Some(new_id);
        }
        let Some(id) = id else { return };
        let gx0 = ((p.x - cell) / cell).floor() as i32;
        let gx1 = ((p.x + cell) / cell).floor() as i32;
        let gy0 = ((p.y - cell) / cell).floor() as i32;
        let gy1 = ((p.y + cell) / cell).floor() as i32;
        let mut cells = Vec::new();
        for gy in gy0..=gy1 {
            for gx in gx0..=gx1 {
                let center = Point::new((gx as f32 + 0.5) * cell, (gy as f32 + 0.5) * cell);
                if center.distance(p) <= cell * 0.9 {
                    if let Some(color) = self.state.doc.sample(center) {
                        cells.push(crate::model::MosaicCell {
                            rect: Rect::new(gx as f32 * cell, gy as f32 * cell, cell, cell),
                            color: color.with_alpha(255),
                        });
                    }
                }
            }
        }
        for c in cells {
            self.state.doc.add_mosaic_cell(id, c);
        }
    }

    fn erase_segment(&mut self, to: Point, tol: f32) {
        let last = match &self.state.drag {
            Drag::Erase { last, .. } => *last,
            _ => return,
        };
        let radius = (self.state.style.brush_size * 0.5).max(2.0);
        let dist = last.distance(to);
        let steps = (dist / (radius * 0.4).max(1.0)).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let p = last.lerp(to, i as f32 / steps as f32);
            self.erase_at(p, tol);
        }
        if let Drag::Erase { last, .. } = &mut self.state.drag {
            *last = to;
        }
    }

    fn erase_at(&mut self, p: Point, tol: f32) {
        let Some(id) = self.state.doc.hit_test(p, tol) else {
            return;
        };
        let pushed = matches!(&self.state.drag, Drag::Erase { pushed: true, .. });
        if !pushed {
            self.state.push_history();
            if let Drag::Erase { pushed, .. } = &mut self.state.drag {
                *pushed = true;
            }
        }
        let radius = (self.state.style.brush_size * 0.5).max(2.0);
        let kind = self.state.doc.layer(id).map(|l| match &l.kind {
            LayerKind::Image { .. } => 0,
            LayerKind::Mosaic { .. } => 1,
            _ => 2,
        });
        match kind {
            Some(0) => {
                if let Some(layer) = self.state.doc.layer_mut(id) {
                    layer.erase_image_alpha(p, radius);
                }
            }
            Some(1) => {
                if let Some(layer) = self.state.doc.layer_mut(id) {
                    layer.erase_mosaic(p, radius);
                }
            }
            Some(2) => {
                self.state.doc.remove(id);
                if self.state.selected == Some(id) {
                    self.state.selected = None;
                }
            }
            _ => {}
        }
    }
}
