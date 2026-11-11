use std::path::PathBuf;
use std::sync::Arc;

use tiny_skia::Pixmap;

use crate::codec::{self, ExportFormat};
use crate::model::canvas::CanvasMode;
use crate::model::layer::{Layer, LayerKind};
use crate::model::{ColorTarget, Rect};
use crate::platform::{dialogs, system};
use crate::render::render_for_export;
use crate::state::{Modal, Popup};
use crate::tools::Tool;

use super::App;

impl App {
    pub(super) fn tool_action(&mut self, tool: Tool) {
        if self.state.crop.is_some() {
            self.state.crop = None;
            self.state.modal = None;
            self.state.toast = None;
        }
        match tool {
            Tool::Open => self.open_dialog(),
            Tool::Paste => self.paste_image(),
            Tool::Delete => self.delete_selected(),
            Tool::Clear => self.clear_canvas(),
            Tool::Undo => self.undo(),
            Tool::Save => self.open_save_modal(),
            _ if tool.is_mode() => {
                self.state.tool = tool;
                self.state.close_popup();
                self.invalidate();
            }
            _ => {}
        }
    }

    pub(super) fn open_dialog(&mut self) {
        let initial = if self.state.last_dir.is_empty() {
            system::exe_dir().to_string_lossy().to_string()
        } else {
            self.state.last_dir.clone()
        };
        let paths = dialogs::open_images(self.hwnd, &initial);
        if paths.is_empty() {
            return;
        }
        if let Some(parent) = paths[0].parent() {
            self.state.last_dir = parent.to_string_lossy().to_string();
        }
        self.open_paths(paths);
    }

    pub(super) fn open_paths(&mut self, paths: Vec<PathBuf>) {
        let mut loaded: Vec<(String, Pixmap)> = Vec::new();
        let mut first_error = String::new();
        for path in paths {
            match codec::load_pixmap(&path) {
                Ok(pixmap) => loaded.push((system::file_stem(&path), pixmap)),
                Err(e) => {
                    if first_error.is_empty() {
                        first_error = format!("{}: {e}", path.file_name().unwrap_or_default().to_string_lossy());
                    }
                }
            }
        }
        if loaded.is_empty() {
            self.state.toast_fmt("status.open_failed", &[&first_error]);
            self.invalidate();
            return;
        }
        let count = loaded.len();
        self.state.push_history();
        let mut last = None;
        for (name, pixmap) in loaded {
            last = Some(self.add_pixmap(pixmap, name));
        }
        self.after_add(last);
        self.state.toast_fmt("status.opened", &[&count.to_string()]);
        self.invalidate();
    }

    fn add_pixmap(&mut self, pixmap: Pixmap, name: String) -> u64 {
        let pw = pixmap.width() as f32;
        let ph = pixmap.height() as f32;
        let page = self.state.doc.page_rect();
        let layers = self.state.doc.layers.len();
        let (x, y, w, h) = if let Some(page) = page {
            if layers == 0 {
                (page.center().x - pw * 0.5, page.center().y - ph * 0.5, pw, ph)
            } else {
                let offset = (layers % 6) as f32 * 16.0;
                (page.x + page.w * 0.06 + offset, page.y + page.h * 0.06 + offset, pw, ph)
            }
        } else if layers == 0 {
            (0.0, 0.0, pw, ph)
        } else {
            let b = self.state.doc.content_bounds().unwrap_or(Rect::new(0.0, 0.0, pw, ph));
            (b.right() + 24.0, b.y, pw, ph)
        };
        let layer = Layer::image(0, name, Arc::new(pixmap), Rect::new(x, y, w, h));
        self.state.dirty = true;
        self.state.doc.add(layer)
    }

    fn after_add(&mut self, selected: Option<u64>) {
        if let Some(id) = selected {
            self.state.selected = Some(id);
        }
        self.state.tool = Tool::Select;
        self.state.view.fitted = false;
        self.fit_current();
        self.invalidate();
    }

    pub(super) fn paste_image(&mut self) {
        let content = match crate::platform::clipboard::read(self.hwnd) {
            Ok(c) => c,
            Err(_) => {
                self.state.toast_i18n("status.clipboard_empty");
                self.invalidate();
                return;
            }
        };
        let mut pixmaps = Vec::new();
        for item in content {
            match item {
                crate::platform::clipboard::ClipboardContent::Encoded(bytes) => {
                    if let Ok(pixmap) = codec::decode_bytes(&bytes) {
                        pixmaps.push((system::file_stem(std::path::Path::new("paste")), pixmap));
                    }
                }
                crate::platform::clipboard::ClipboardContent::Files(paths) => {
                    for path in paths {
                        if let Ok(pixmap) = codec::load_pixmap(&path) {
                            pixmaps.push((system::file_stem(&path), pixmap));
                        }
                    }
                }
            }
        }
        if pixmaps.is_empty() {
            self.state.toast_i18n("status.clipboard_empty");
            self.invalidate();
            return;
        }
        self.state.push_history();
        let mut last = None;
        for (name, pixmap) in pixmaps {
            last = Some(self.add_pixmap(pixmap, name));
        }
        self.after_add(last);
        self.state.toast_i18n("status.pasted");
        self.invalidate();
    }

    pub(super) fn delete_selected(&mut self) {
        let Some(id) = self.state.selected else {
            self.state.toast_i18n("status.no_selection");
            self.invalidate();
            return;
        };
        self.state.push_history();
        self.state.doc.remove(id);
        self.state.selected = None;
        self.state.toast_i18n("status.deleted");
        self.invalidate();
    }

    pub(super) fn clear_canvas(&mut self) {
        if self.state.doc.layers.is_empty() {
            return;
        }
        self.state.push_history();
        self.state.doc.clear();
        self.state.selected = None;
        self.state.toast_i18n("status.cleared");
        self.invalidate();
    }

    pub(super) fn undo(&mut self) {
        if self.state.undo() {
            self.state.toast_i18n("status.undone");
        } else {
            self.state.toast_i18n("status.no_undo");
        }
        self.invalidate();
    }

    pub(super) fn open_save_modal(&mut self) {
        self.state.close_after_save = false;
        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }
        if self.state.page_pixels().is_none() {
            self.state.toast_i18n("status.nothing");
            self.invalidate();
            return;
        }
        self.state.close_popup();
        self.state.modal = Some(Modal::Save);
        self.invalidate();
    }

    pub(super) fn export_image(&mut self) {
        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }
        let Some(_) = self.state.page_pixels() else {
            self.state.toast_i18n("status.nothing");
            return;
        };
        let initial = if self.state.last_dir.is_empty() {
            system::exe_dir().to_string_lossy().to_string()
        } else {
            self.state.last_dir.clone()
        };
        let default_name = system::default_export_name(self.state.save_format.as_str());
        let Some(path) = dialogs::save_image(
            self.hwnd,
            &initial,
            &default_name,
            self.state.save_format.as_str(),
        ) else {
            self.state.close_after_save = false;
            return;
        };
        let format = ExportFormat::from_path(&path).unwrap_or(self.state.save_format);
        let transparent = self.state.save_transparent && format.supports_alpha();
        let result = render_for_export(&self.state.doc, format, transparent, &mut self.renderer.text)
            .and_then(|img| {
                codec::write_image(
                    &path,
                    format,
                    &img.pixmap,
                    if format.quality_enabled() {
                        self.state.save_quality
                    } else {
                        100
                    },
                )
            });
        match result {
            Ok(bytes) => {
                if let Some(parent) = path.parent() {
                    self.state.last_dir = parent.to_string_lossy().to_string();
                }
                self.state.modal = None;
                self.state.slider_drag = false;
                self.state.dirty = false;
                let close_after = self.state.close_after_save;
                self.state.close_after_save = false;
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let summary = format!("{name} · {}", system::format_bytes(bytes));
                self.state.toast_fmt("status.saved", &[&summary]);
                if close_after {
                    self.close_now();
                    return;
                }
            }
            Err(e) => {
                self.state.close_after_save = false;
                self.state.toast_fmt("status.save_failed", &[&e]);
            }
        }
        self.invalidate();
    }

    pub(super) fn apply_color(&mut self, color: crate::model::Rgba) {
        let Some(Popup::Color { target, .. }) = self.state.popup.clone() else {
            return;
        };
        let selected = self.state.selected;
        let mut changed = false;
        if let Some(id) = selected {
            if let Some(layer) = self.state.doc.layer(id) {
                changed = match target {
                    ColorTarget::Border => {
                        matches!(layer.kind, LayerKind::Shape { .. } | LayerKind::Stroke { .. })
                    }
                    ColorTarget::Fill => matches!(layer.kind, LayerKind::Shape { .. }),
                    ColorTarget::Text => matches!(layer.kind, LayerKind::Text { .. }),
                    ColorTarget::TextOutline => matches!(layer.kind, LayerKind::Text { .. }),
                };
            }
        }
        if changed {
            self.state.push_history();
            if let Some(layer) = self.state.selected_layer_mut() {
                match target {
                    ColorTarget::Border => {
                        if matches!(layer.kind, LayerKind::Shape { .. }) {
                            layer.set_shape_style(Some(color), None, None);
                        } else if matches!(layer.kind, LayerKind::Stroke { .. }) {
                            layer.set_stroke_style(Some(color), None);
                        }
                    }
                    ColorTarget::Fill => {
                        layer.set_shape_style(None, Some(Some(color)), None);
                    }
                    ColorTarget::Text => layer.set_text_color(color),
                    ColorTarget::TextOutline => layer.set_text_outline(Some(color)),
                }
            }
        }
        match target {
            ColorTarget::Border => self.state.style.border = color,
            ColorTarget::Fill => self.state.style.fill = Some(color),
            ColorTarget::Text => self.state.style.text = color,
            ColorTarget::TextOutline => self.state.style.text_outline = Some(color),
        }
        if target == ColorTarget::TextOutline {
            if let Some(edit) = &mut self.state.text_edit {
                edit.outline = Some(color);
            }
        }
        if target == ColorTarget::Text {
            if let Some(edit) = &mut self.state.text_edit {
                edit.color = color;
            }
        }
        self.state.close_popup();
        self.invalidate();
    }

    pub(super) fn apply_fill_none(&mut self) {
        let selected = self.state.selected;
        let changed = selected
            .and_then(|id| self.state.doc.layer(id))
            .map(|l| matches!(l.kind, LayerKind::Shape { .. }))
            .unwrap_or(false);
        if changed {
            self.state.push_history();
            if let Some(layer) = self.state.selected_layer_mut() {
                layer.set_shape_style(None, Some(None), None);
            }
        }
        self.state.style.fill = None;
        self.state.close_popup();
        self.invalidate();
    }

    pub(super) fn apply_color_none(&mut self) {
        match self.state.popup.clone() {
            Some(Popup::Color {
                target: ColorTarget::TextOutline,
                ..
            }) => self.clear_text_outline(),
            _ => self.apply_fill_none(),
        }
    }

    pub(super) fn choose_custom_color(&mut self) {
        let target = match self.state.popup.clone() {
            Some(Popup::Color { target, .. }) => target,
            _ => return,
        };
        let initial = match target {
            ColorTarget::Border => self.state.style.border,
            ColorTarget::Fill => self.state.style.fill.unwrap_or(crate::model::Rgba::WHITE),
            ColorTarget::Text => self.state.style.text,
            ColorTarget::TextOutline => self
                .state
                .style
                .text_outline
                .unwrap_or(crate::model::Rgba::WHITE),
        };
        if let Some(color) = dialogs::choose_color(self.hwnd, initial, &mut self.custom_colors) {
            self.apply_color(color);
        }
    }

    pub(super) fn adjust_font(&mut self, delta: f32) {
        let old = self.state.style.font_size;
        let new = (old + delta).clamp(6.0, 400.0);
        if (new - old).abs() < f32::EPSILON {
            return;
        }
        let text_layer = self
            .state
            .selected_layer()
            .and_then(|l| l.text_value().map(|s| s.to_string()));
        if let Some(text) = text_layer {
            let family = self
                .state
                .selected_layer()
                .and_then(|l| l.text_family().map(|s| s.to_string()))
                .unwrap_or_else(|| self.state.style.font_family.clone());
            let (w, h) = self.renderer.text.measure_family(&text, new, 400, &family);
            self.state.push_history();
            if let Some(layer) = self.state.selected_layer_mut() {
                if let LayerKind::Text { font_size, .. } = &mut layer.kind {
                    *font_size = new;
                    layer.rect.w = w;
                    layer.rect.h = h;
                }
            }
        }
        self.state.style.font_size = new;
        self.invalidate();
    }

    pub(super) fn set_font_family(&mut self, family: &str) {
        let selected = self.state.selected.and_then(|id| {
            self.state.doc.layer(id).and_then(|layer| {
                layer.text_value().map(|text| {
                    (
                        text.to_string(),
                        layer.text_style().map(|(_, size)| size).unwrap_or(self.state.style.font_size),
                        layer.text_outline(),
                        layer
                            .text_outline_width()
                            .unwrap_or(self.state.style.text_outline_width),
                    )
                })
            })
        });
        if let Some((text, size, outline, out_width)) = selected {
            let (w, h) = self.renderer.text.measure_family(&text, size, 400, family);
            self.state.push_history();
            if let Some(layer) = self.state.selected_layer_mut() {
                layer.rect.w = w;
                layer.rect.h = h;
                layer.set_text_style(family, outline, out_width);
            }
        }
        self.state.style.font_family = family.to_string();
        if let Some(edit) = &mut self.state.text_edit {
            edit.font_family = family.to_string();
        }
        self.state.close_popup();
        self.invalidate();
    }

    pub(super) fn clear_text_outline(&mut self) {
        let has_text = self
            .state
            .selected_layer()
            .map(|layer| matches!(layer.kind, LayerKind::Text { .. }))
            .unwrap_or(false);
        if has_text {
            self.state.push_history();
        }
        self.state.style.text_outline = None;
        if let Some(edit) = &mut self.state.text_edit {
            edit.outline = None;
        }
        if let Some(layer) = self.state.selected_layer_mut() {
            if matches!(layer.kind, LayerKind::Text { .. }) {
                layer.set_text_outline(None);
            }
        }
        self.state.close_popup();
        self.invalidate();
    }

    pub(super) fn adjust_stroke(&mut self, delta: f32) {
        let new = (self.state.style.stroke_width + delta).clamp(0.5, 80.0);
        self.state.style.stroke_width = new;
        self.state.style.text_outline_width = new;
        if let Some(edit) = &mut self.state.text_edit {
            edit.outline_width = new;
        }
        if let Some(id) = self.state.selected {
            let kind = self.state.doc.layer(id).map(|l| match l.kind {
                LayerKind::Shape { .. } => 1,
                LayerKind::Stroke { .. } => 2,
                LayerKind::Text { .. } => 3,
                _ => 0,
            });
            match kind {
                Some(1) => {
                    self.state.push_history();
                    if let Some(layer) = self.state.selected_layer_mut() {
                        layer.set_shape_style(None, None, Some(new));
                    }
                }
                Some(2) => {
                    self.state.push_history();
                    if let Some(layer) = self.state.selected_layer_mut() {
                        layer.set_stroke_style(None, Some(new));
                    }
                }
                Some(3) => {
                    self.state.push_history();
                    if let Some(layer) = self.state.selected_layer_mut() {
                        layer.set_text_outline_width(new);
                    }
                }
                _ => {}
            }
        }
        self.invalidate();
    }

    pub(super) fn adjust_brush(&mut self, delta: f32) {
        self.state.style.brush_size = (self.state.style.brush_size + delta).clamp(1.0, 200.0);
        self.invalidate();
    }

    pub(super) fn adjust_mosaic(&mut self, delta: f32) {
        self.state.style.mosaic_size = (self.state.style.mosaic_size + delta).clamp(3.0, 100.0);
        self.invalidate();
    }

    pub(super) fn apply_canvas(&mut self, mode: CanvasMode) {
        self.state.push_history();
        self.state.doc.canvas = mode.clone();
        self.state.popup = None;
        self.state.canvas_form = crate::state::CanvasForm::from_mode(&mode);
        self.state.view.fitted = false;
        self.fit_current();
        match mode.pixel_size() {
            Some((w, h)) => {
                self.state.toast_fmt("status.canvas_fixed", &[&w.to_string(), &h.to_string()]);
            }
            None => self.state.toast_i18n("status.canvas_auto"),
        }
        self.invalidate();
    }

    pub(super) fn apply_custom_canvas(&mut self) {
        match self.state.canvas_form.parse() {
            Some(mode) => self.apply_canvas(mode),
            None => {
                self.state.canvas_form.invalid = true;
                self.invalidate();
            }
        }
    }

    pub(super) fn set_language(&mut self, code: &str) {
        let resolved = if code == "auto" {
            system::system_language()
        } else {
            code.to_string()
        };
        if self.state.i18n.set(&resolved) {
            self.state.lang_pref = code.to_string();
            self.state.popup = None;
            self.save_settings();
            self.invalidate();
        }
    }
}
