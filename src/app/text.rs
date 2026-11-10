use std::ffi::c_void;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, GetWindowTextLengthW, GetWindowTextW,
};

use crate::model::layer::{Layer, LayerKind};
use crate::model::{Point, Rect};
use crate::state::TextEditState;

use super::App;

const VK_ESCAPE: usize = 0x1B;
const VK_RETURN: usize = 0x0D;

impl App {
    pub(super) fn begin_new_text(&mut self, pos: Point) {
        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }
        self.state.text_edit = Some(TextEditState {
            layer_id: None,
            original: String::new(),
            color: self.state.style.text,
            font_size: self.state.style.font_size,
            font_family: self.state.style.font_family.clone(),
            outline: self.state.style.text_outline,
            outline_width: self.state.style.text_outline_width,
            pos,
            screen_rect: Rect::ZERO,
        });
        self.create_editor_from_state();
        self.state.tool = crate::tools::Tool::Text;
        self.invalidate();
    }

    pub(super) fn begin_edit_text(&mut self, id: u64) {
        let Some(layer) = self.state.doc.layer(id) else {
            return;
        };
        let Some(text) = layer.text_value().map(|s| s.to_string()) else {
            return;
        };
        let (color, font_size) = layer
            .text_style()
            .unwrap_or((self.state.style.text, self.state.style.font_size));
        let font_family = layer
            .text_family()
            .unwrap_or(&self.state.style.font_family)
            .to_string();
        let outline = layer.text_outline();
        let outline_width = layer
            .text_outline_width()
            .unwrap_or(self.state.style.text_outline_width);
        let pos = Point::new(layer.rect.x, layer.rect.y);
        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }
        self.state.text_edit = Some(TextEditState {
            layer_id: Some(id),
            original: text,
            color,
            font_size,
            font_family,
            outline,
            outline_width,
            pos,
            screen_rect: Rect::ZERO,
        });
        self.create_editor_from_state();
        self.state.selected = Some(id);
        self.invalidate();
    }

    fn create_editor_from_state(&mut self) {
        let Some(edit) = self.state.text_edit.clone() else {
            return;
        };
        let scale = (self.dpi / 96.0).clamp(0.75, 3.0);
        let zoom = self.state.view.zoom.max(0.02);
        let font_screen = (edit.font_size * zoom).clamp(9.0, 80.0);
        let (tw, th) = self.renderer.text.measure_family(
            if edit.original.is_empty() { "输入文字" } else { &edit.original },
            font_screen,
            400,
            &edit.font_family,
        );
        let width = (tw + 50.0 * scale).max(300.0 * scale).min(self.layout.canvas.w - 20.0 * scale);
        let height = (th + 46.0 * scale).max(120.0 * scale).min(self.layout.canvas.h - 20.0 * scale);
        let mut x = edit.pos.x * zoom + self.state.view.pan.x;
        let mut y = edit.pos.y * zoom + self.state.view.pan.y;
        x = x.clamp(self.layout.canvas.x + 6.0, self.layout.canvas.right() - width - 6.0);
        y = y.clamp(self.layout.canvas.y + 6.0, self.layout.canvas.bottom() - height - 6.0);
        let rect = Rect::new(x, y, width, height);
        if let Some(state) = &mut self.state.text_edit {
            state.screen_rect = rect;
        }
        let font = self
            .renderer
            .text
            .font_family(font_screen * crate::platform::gdi::TEXT_SCALE, 400, &edit.font_family);
        let hwnd = crate::platform::window::create_edit_control(
            self.hwnd,
            rect,
            font as *mut c_void,
            &edit.original,
        );
        if hwnd.is_null() {
            self.state.text_edit = None;
            return;
        }
        self.editor_hwnd = hwnd;
    }

    pub fn finish_text_edit(&mut self, commit: bool) {
        if self.closing_editor {
            return;
        }
        let edit = self.state.text_edit.take();
        let hwnd = self.editor_hwnd;
        let mut value = String::new();
        if !hwnd.is_null() {
            unsafe {
                let len = GetWindowTextLengthW(hwnd).max(0);
                let mut buf = vec![0u16; len as usize + 1];
                GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
                value = crate::platform::system::from_wide(&buf);
            }
        }
        if !hwnd.is_null() {
            self.closing_editor = true;
            unsafe {
                DestroyWindow(hwnd);
            }
            self.editor_hwnd = std::ptr::null_mut();
            self.closing_editor = false;
        }
        self.focus_main();
        let Some(edit) = edit else {
            self.invalidate();
            return;
        };
        if !commit {
            self.invalidate();
            return;
        }
        let value = value.trim_end_matches(|c| c == '\r' || c == '\n').to_string();
        let existing_id = edit.layer_id;
        if value.trim().is_empty() {
            if let Some(id) = existing_id {
                if self.state.doc.layer(id).is_some() {
                    self.state.push_history();
                    self.state.doc.remove(id);
                    if self.state.selected == Some(id) {
                        self.state.selected = None;
                    }
                }
            }
            self.invalidate();
            return;
        }
        let (w, h) = self
            .renderer
            .text
            .measure_family(&value, edit.font_size, 400, &edit.font_family);
        let rect = Rect::new(edit.pos.x, edit.pos.y, w, h);
        if let Some(id) = existing_id {
            if let Some(layer) = self.state.doc.layer(id) {
                if layer.text_value() == Some(value.as_str())
                    && layer.text_family() == Some(edit.font_family.as_str())
                    && layer.text_outline() == edit.outline
                    && layer.text_outline_width() == Some(edit.outline_width)
                {
                    self.state.selected = Some(id);
                    self.invalidate();
                    return;
                }
            }
            self.state.push_history();
            if let Some(layer) = self.state.doc.layer_mut(id) {
                if matches!(layer.kind, LayerKind::Text { .. }) {
                    layer.set_text(value, rect, edit.font_size);
                    layer.set_text_color(edit.color);
                    layer.set_text_style(&edit.font_family, edit.outline, edit.outline_width);
                } else {
                    self.add_text_layer(
                        value,
                        edit.color,
                        edit.font_size,
                        &edit.font_family,
                        edit.outline,
                        edit.outline_width,
                        rect,
                    );
                }
            } else {
                self.add_text_layer(
                    value,
                    edit.color,
                    edit.font_size,
                    &edit.font_family,
                    edit.outline,
                    edit.outline_width,
                    rect,
                );
            }
            self.state.selected = Some(id);
        } else {
            let id = self.add_text_layer(
                value,
                edit.color,
                edit.font_size,
                &edit.font_family,
                edit.outline,
                edit.outline_width,
                rect,
            );
            self.state.selected = Some(id);
        }
        self.state.tool = crate::tools::Tool::Select;
        self.invalidate();
    }

    fn add_text_layer(
        &mut self,
        text: String,
        color: crate::model::Rgba,
        font_size: f32,
        font_family: &str,
        outline: Option<crate::model::Rgba>,
        outline_width: f32,
        rect: Rect,
    ) -> u64 {
        let layer = Layer::text_styled(
            0,
            text,
            color,
            font_size,
            font_family,
            outline,
            outline_width,
            rect,
        );
        self.state.dirty = true;
        self.state.doc.add(layer)
    }

    pub fn on_editor_key(&mut self, vk: usize, ctrl: bool) -> bool {
        if vk == VK_ESCAPE {
            self.finish_text_edit(false);
            return true;
        }
        if vk == VK_RETURN && ctrl {
            self.finish_text_edit(true);
            return true;
        }
        false
    }

    pub fn editor_rect(&self) -> Option<Rect> {
        self.state.text_edit.as_ref().map(|e| e.screen_rect)
    }
}

pub fn _unused(_: HWND, _: *mut c_void) {}
