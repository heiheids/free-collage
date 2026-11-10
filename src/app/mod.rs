mod actions;
mod input;
mod text;

use std::ffi::c_void;

use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT,
    HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, KillTimer, PostMessageW, SetTimer, ShowWindow,
    SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
};

use crate::config::Settings;
use crate::i18n::I18n;
use crate::model::ColorTarget;
use crate::model::{Rect as DRect, Rgba};
use crate::platform::gdi;
use crate::platform::system::{self, settings_path};
use crate::render::{Frame, Renderer};
use crate::state::{Modal, Popup, State};
use crate::tools::{Cursor, Tool};
use crate::ui::{Hit, HitId, Layout};

pub struct App {
    pub hwnd: HWND,
    pub hinstance: HINSTANCE,
    pub dpi: f32,
    pub client_w: i32,
    pub client_h: i32,
    pub state: State,
    pub renderer: Renderer,
    pub layout: Layout,
    pub hits: Vec<Hit>,
    pub buffer: Vec<u32>,
    pub editor_hwnd: HWND,
    pub closing_editor: bool,
    pub custom_colors: [u32; 16],
    pub mouse_down: bool,
    pub force_close: bool,
}

type HINSTANCE = *mut c_void;

impl App {
    pub fn new(hwnd: HWND, hinstance: HINSTANCE) -> Self {
        let settings = Settings::load(&settings_path());
        let system_lang = system::system_language();
        let i18n = I18n::load(&settings.language, &system_lang);
        let state = State::new(settings, i18n);
        let mut app = Self {
            hwnd,
            hinstance,
            dpi: system::window_dpi(hwnd),
            client_w: 1280,
            client_h: 840,
            state,
            renderer: Renderer::new(),
            layout: Layout::new(1280.0, 840.0, 1.0),
            hits: Vec::new(),
            buffer: Vec::new(),
            editor_hwnd: std::ptr::null_mut(),
            closing_editor: false,
            custom_colors: [0xFFFFFF; 16],
            mouse_down: false,
            force_close: false,
        };
        unsafe {
            windows_sys::Win32::UI::Shell::DragAcceptFiles(hwnd, 1);
            SetTimer(hwnd, 1, 300, None);
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rc);
            app.resize((rc.right - rc.left).max(1), (rc.bottom - rc.top).max(1));
        }
        app
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        if (w, h) == (self.client_w, self.client_h) {
            return;
        }
        if self.editor_hwnd != std::ptr::null_mut() {
            self.finish_text_edit(true);
        }
        self.client_w = w.max(1);
        self.client_h = h.max(1);
        let scale = (self.dpi / 96.0).clamp(0.75, 3.0);
        self.layout = Layout::new(self.client_w as f32, self.client_h as f32, scale);
        self.state.is_maximized = unsafe { IsZoomed(self.hwnd) != 0 };
        if !self.state.view.fitted {
            self.fit_current();
        }
        self.invalidate();
    }

    pub fn paint(&mut self, hdc: *mut c_void) {
        self.sync_client_size();
        let frame: Frame = self
            .renderer
            .render(&self.state, self.client_w, self.client_h, self.dpi);
        let Frame {
            pixmap,
            layout,
            hits,
        } = frame;
        self.layout = layout;
        self.hits = hits;
        gdi::present(
            hdc,
            self.client_w,
            self.client_h,
            &pixmap,
            &mut self.buffer,
        );
        self.renderer.recycle(pixmap);
    }

    fn sync_client_size(&mut self) {
        unsafe {
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(self.hwnd, &mut rc);
            let w = (rc.right - rc.left).max(1);
            let h = (rc.bottom - rc.top).max(1);
            let dpi = system::window_dpi(self.hwnd);
            if (dpi - self.dpi).abs() > 0.1 {
                self.dpi = dpi;
                self.renderer.text.clear_masks();
                self.state.view.fitted = false;
            }
            if (w, h) != (self.client_w, self.client_h) {
                self.client_w = w;
                self.client_h = h;
                let scale = (self.dpi / 96.0).clamp(0.75, 3.0);
                self.layout = Layout::new(w as f32, h as f32, scale);
                if !self.state.view.fitted {
                    let padding = 48.0 * scale;
                    self.state.fit(self.layout.canvas, padding);
                }
            }
        }
    }

    pub fn invalidate(&self) {
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn on_dpi_changed(&mut self, dpi: f32) {
        self.dpi = dpi;
        self.renderer.text.clear_masks();
        self.state.view.fitted = false;
        let scale = (self.dpi / 96.0).clamp(0.75, 3.0);
        self.layout = Layout::new(self.client_w as f32, self.client_h as f32, scale);
        self.invalidate();
    }

    pub fn on_timer(&mut self) {
        if let Some(toast) = &self.state.toast {
            if std::time::Instant::now() >= toast.until {
                self.state.toast = None;
                self.invalidate();
            }
        }
    }

    pub fn on_destroy(&mut self) {
        unsafe {
            KillTimer(self.hwnd, 1);
        }
        if self.editor_hwnd != std::ptr::null_mut() {
            self.finish_text_edit(false);
        }
        self.save_settings();
    }

    pub fn save_settings(&self) {
        let settings = self.state.to_settings();
        let _ = settings.save(&settings_path());
    }

    pub fn on_drop(&mut self, paths: Vec<std::path::PathBuf>) {
        self.open_paths(paths);
    }

    pub fn paste_clipboard(&mut self) {
        self.paste_image();
    }

    pub fn dump_state(&mut self) {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(
            s,
            "dpi={} client={}x{} scale={:.3} view.zoom={:.4} view.pan=({:.2},{:.2}) tool={:?} selected={:?} mouse=({:.1},{:.1})",
            self.dpi,
            self.client_w,
            self.client_h,
            self.dpi / 96.0,
            self.state.view.zoom,
            self.state.view.pan.x,
            self.state.view.pan.y,
            self.state.tool,
            self.state.selected,
            self.state.mouse.x,
            self.state.mouse.y
        );
        let _ = writeln!(
            s,
            "layout.canvas=({:.1},{:.1},{:.1},{:.1}) bottom_y={:.1}",
            self.layout.canvas.x,
            self.layout.canvas.y,
            self.layout.canvas.w,
            self.layout.canvas.h,
            self.layout.bottom.y
        );
        let _ = writeln!(s, "canvas={:?}", self.state.doc.canvas);
        for layer in &self.state.doc.layers {
            let kind = match &layer.kind {
                crate::model::LayerKind::Image { pixmap } => format!("image {}x{}", pixmap.width(), pixmap.height()),
                crate::model::LayerKind::Text { text, font_size, .. } => format!("text {text:?} size={font_size}"),
                crate::model::LayerKind::Stroke { points, width, .. } => format!("stroke n={} width={width}", points.len()),
                crate::model::LayerKind::Shape { kind, a, b, .. } => format!("shape {:?} a=({:.1},{:.1}) b=({:.1},{:.1})", kind, a.x, a.y, b.x, b.y),
                crate::model::LayerKind::Mosaic { cells, .. } => format!("mosaic n={}", cells.len()),
            };
            let _ = writeln!(
                s,
                "layer id={} {kind} rect=({:.2},{:.2},{:.2},{:.2})",
                layer.id, layer.rect.x, layer.rect.y, layer.rect.w, layer.rect.h
            );
        }
        let path = system::exe_dir().join("state-dump.txt");
        let _ = std::fs::write(&path, s);
        self.state.toast(format!("state: {}", path.display()));
        self.invalidate();
    }

    pub fn command(&mut self, control: HWND, code: u32) {
        if control == self.editor_hwnd {
            const EN_CHANGE: u32 = 0x0300;
            const EN_KILLFOCUS: u32 = 0x0200;
            if code == EN_KILLFOCUS && !self.closing_editor {
                self.finish_text_edit(true);
            } else if code == EN_CHANGE {
                self.invalidate();
            }
        }
    }

    pub fn set_maximized(&mut self, value: bool) {
        if self.state.is_maximized != value {
            self.state.is_maximized = value;
            self.invalidate();
        }
    }

    pub fn cursor(&self) -> Cursor {
        self.state.cursor
    }

    pub fn focus_main(&self) {
        unsafe {
            SetFocus(self.hwnd);
        }
    }

    pub fn nonclient_hit_test(&self, x: i32, y: i32) -> isize {
        let scale = (self.dpi / 96.0).clamp(0.75, 3.0);
        let bw = (7.0 * scale) as i32;
        let w = self.client_w;
        let h = self.client_h;
        if !self.state.is_maximized {
            let left = x < bw;
            let right = x >= w - bw;
            let top = y < bw;
            let bottom = y >= h - bw;
            if top && left {
                return HTTOPLEFT as isize;
            }
            if top && right {
                return HTTOPRIGHT as isize;
            }
            if bottom && left {
                return HTBOTTOMLEFT as isize;
            }
            if bottom && right {
                return HTBOTTOMRIGHT as isize;
            }
            if left {
                return HTLEFT as isize;
            }
            if right {
                return HTRIGHT as isize;
            }
            if top {
                return HTTOP as isize;
            }
            if bottom {
                return HTBOTTOM as isize;
            }
        }

        if (y as f32) < self.layout.header.bottom() {
            let p = DRect::new(x as f32, y as f32, 1.0, 1.0).center();
            for rect in [
                self.layout.canvas_chip,
                self.layout.screenshot,
                self.layout.about,
                self.layout.lang,
                self.layout.min,
                self.layout.max,
                self.layout.close,
            ] {
                if rect.inflate(2.0).contains(p) {
                    return HTCLIENT as isize;
                }
            }
            return HTCAPTION as isize;
        }
        HTCLIENT as isize
    }

    pub fn request_close(&mut self) {
        if self.force_close {
            return;
        }
        if matches!(self.state.modal, Some(Modal::CloseConfirm)) {
            return;
        }
        if matches!(self.state.modal, Some(Modal::Save)) {
            self.state.modal = None;
            self.state.slider_drag = false;
            self.state.close_after_save = false;
            self.invalidate();
            return;
        }
        if self.state.modal.is_some() {
            self.state.modal = None;
        }
        if self.state.text_edit.is_some() {
            self.finish_text_edit(true);
        }
        let needs_prompt = self.state.dirty && self.state.page_pixels().is_some();
        if !needs_prompt {
            self.close_now();
            return;
        }
        self.state.popup = None;
        self.state.hover = None;
        self.state.modal = Some(Modal::CloseConfirm);
        self.invalidate();
    }

    pub fn close_now(&mut self) {
        self.force_close = true;
        unsafe {
            PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
        }
    }

    fn start_capture(&mut self) {
        crate::platform::capture::start(self.hwnd);
    }

    pub fn window_command(&mut self, id: HitId) {
        unsafe {
            match id {
                HitId::Min => {
                    ShowWindow(self.hwnd, SW_MINIMIZE);
                }
                HitId::Max => {
                    if IsZoomed(self.hwnd) != 0 {
                        ShowWindow(self.hwnd, SW_RESTORE);
                    } else {
                        ShowWindow(self.hwnd, SW_MAXIMIZE);
                    }
                }
                HitId::Close => {
                    PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
                }
                _ => {}
            }
        }
    }

    fn toggle_color_popup(&mut self, target: ColorTarget) {
        if matches!(
            &self.state.popup,
            Some(Popup::Color { target: t, .. }) if *t == target
        ) {
            self.state.close_popup();
        } else {
            let anchor = match target {
                ColorTarget::Border => self.layout.border_color,
                ColorTarget::Fill => self.layout.fill_color,
                ColorTarget::Text => self.layout.text_color,
                ColorTarget::TextOutline => self.layout.font_family,
            };
            self.state.popup = Some(Popup::Color { target, anchor });
        }
        self.invalidate();
    }

    pub fn open_canvas_popup(&mut self) {
        if matches!(self.state.popup, Some(Popup::Canvas { .. })) {
            self.state.close_popup();
        } else {
            self.state.canvas_form = crate::state::CanvasForm::from_mode(&self.state.doc.canvas);
            let custom = matches!(
                self.state.doc.canvas,
                crate::model::CanvasMode::Fixed { width_mm, height_mm, .. }
                    if !((width_mm - 210.0).abs() < 0.6 && (height_mm - 297.0).abs() < 0.6)
                        && !((width_mm - 176.0).abs() < 0.6 && (height_mm - 250.0).abs() < 0.6)
            );
            self.state.popup = Some(Popup::Canvas {
                anchor: self.layout.canvas_chip,
                custom,
            });
        }
        self.invalidate();
    }

    pub fn open_language_popup(&mut self) {
        if matches!(self.state.popup, Some(Popup::Language { .. })) {
            self.state.close_popup();
        } else {
            self.state.popup = Some(Popup::Language {
                anchor: self.layout.lang,
            });
        }
        self.invalidate();
    }

    pub fn open_font_popup(&mut self) {
        if matches!(self.state.popup, Some(Popup::Font { .. })) {
            self.state.close_popup();
        } else {
            self.state.popup = Some(Popup::Font {
                anchor: self.layout.font_family,
            });
        }
        self.invalidate();
    }

    fn open_text_outline_color(&mut self) {
        self.state.popup = Some(Popup::Color {
            target: ColorTarget::TextOutline,
            anchor: self.layout.font_family,
        });
        self.invalidate();
    }

    fn move_layer(&mut self, id: u64, up: bool) {
        let Some(index) = self.state.doc.layers.iter().position(|layer| layer.id == id) else {
            return;
        };
        let can_move = if up {
            index + 1 < self.state.doc.layers.len()
        } else {
            index > 0
        };
        if !can_move {
            return;
        }
        self.state.push_history();
        if self.state.doc.move_layer(id, up) {
            self.state.selected = Some(id);
            self.state.toast_i18n(if up {
                "layer.move_up"
            } else {
                "layer.move_down"
            });
        }
        self.state.close_popup();
        self.invalidate();
    }

    pub fn open_layer_menu(&mut self, id: u64, anchor: crate::model::Rect) {
        self.state.popup = Some(Popup::LayerMenu { anchor, id });
        self.state.selected = Some(id);
        self.invalidate();
    }

    pub fn handle_hit(&mut self, id: HitId) {
        match id {
            HitId::Tool(tool) => self.tool_action(tool),
            HitId::Min | HitId::Max | HitId::Close => self.window_command(id),
            HitId::About => {
                self.state.popup = None;
                self.state.modal = Some(Modal::About);
                self.invalidate();
            }
            HitId::Language => self.open_language_popup(),
            HitId::CanvasChip => self.open_canvas_popup(),
            HitId::Screenshot => self.start_capture(),
            HitId::FontFamily => self.open_font_popup(),
            HitId::FontChoice(i) => {
                if let Some((family, _)) = crate::model::style::FONT_CHOICES.get(i as usize) {
                    self.set_font_family(family);
                }
            }
            HitId::TextOutlineColor => self.open_text_outline_color(),
            HitId::LayerUp => {
                if let Some(Popup::LayerMenu { id, .. }) = self.state.popup.clone() {
                    self.move_layer(id, true);
                }
            }
            HitId::LayerDown => {
                if let Some(Popup::LayerMenu { id, .. }) = self.state.popup.clone() {
                    self.move_layer(id, false);
                }
            }
            HitId::BorderColor => self.toggle_color_popup(ColorTarget::Border),
            HitId::FillColor => self.toggle_color_popup(ColorTarget::Fill),
            HitId::TextColor => self.toggle_color_popup(ColorTarget::Text),
            HitId::FontMinus => self.adjust_font(-2.0),
            HitId::FontPlus => self.adjust_font(2.0),
            HitId::StrokeMinus => self.adjust_stroke(-0.5),
            HitId::StrokePlus => self.adjust_stroke(0.5),
            HitId::BrushMinus => self.adjust_brush(-1.0),
            HitId::BrushPlus => self.adjust_brush(1.0),
            HitId::MosaicMinus => self.adjust_mosaic(-1.0),
            HitId::MosaicPlus => self.adjust_mosaic(1.0),
            HitId::ZoomOut => self.zoom_step(1.0 / 1.2),
            HitId::ZoomIn => self.zoom_step(1.2),
            HitId::ZoomFit => {
                self.state.view.fitted = false;
                self.fit_current();
                self.state.toast_i18n("status.fit");
            }
            HitId::SaveFormat(i) => {
                if let Some(f) = crate::codec::ExportFormat::ALL.get(i as usize) {
                    self.state.save_format = *f;
                    self.invalidate();
                }
            }
            HitId::SaveQuality => {
                self.state.slider_drag = true;
                self.set_quality_from_x(self.state.mouse.x);
            }
            HitId::SaveTransparent => {
                if self.state.save_format.supports_alpha() {
                    self.state.save_transparent = !self.state.save_transparent;
                    self.invalidate();
                }
            }
            HitId::SaveExport => self.export_image(),
            HitId::SaveCancel => {
                self.state.modal = None;
                self.state.slider_drag = false;
                self.state.close_after_save = false;
                self.invalidate();
            }
            HitId::CloseSave => {
                self.open_save_modal();
                if matches!(self.state.modal, Some(Modal::Save)) {
                    self.state.close_after_save = true;
                }
            }
            HitId::CloseDiscard => self.close_now(),
            HitId::CloseCancel => {
                self.state.modal = None;
                self.state.close_after_save = false;
                self.invalidate();
            }
            HitId::AboutClose => {
                self.state.modal = None;
                self.invalidate();
            }
            HitId::Homepage => {
                system::open_url("https://space.bilibili.com/19428259");
            }
            HitId::Swatch(i) => {
                if let Some(color) = crate::model::style::PALETTE.get(i as usize).copied() {
                    self.apply_color(color);
                }
            }
            HitId::ColorNone => self.apply_color_none(),
            HitId::ColorCustom => self.choose_custom_color(),
            HitId::CanvasAuto => self.apply_canvas(crate::model::CanvasMode::Auto),
            HitId::CanvasA4 => self.apply_canvas(crate::model::CanvasMode::fixed_a4()),
            HitId::CanvasA4Landscape => self.apply_canvas(crate::model::CanvasMode::fixed_a4_landscape()),
            HitId::CanvasB5 => self.apply_canvas(crate::model::CanvasMode::fixed_b5()),
            HitId::CanvasB5Landscape => self.apply_canvas(crate::model::CanvasMode::fixed_b5_landscape()),
            HitId::CanvasCustom => {
                if let Some(Popup::Canvas { custom, .. }) = &mut self.state.popup {
                    *custom = true;
                }
                self.state.canvas_form = crate::state::CanvasForm::from_mode(&self.state.doc.canvas);
                self.state.canvas_form.focus = 0;
                self.invalidate();
            }
            HitId::CanvasField(i) => {
                self.state.canvas_form.focus = i;
                self.state.canvas_form.invalid = false;
                self.invalidate();
            }
            HitId::CanvasApply => self.apply_custom_canvas(),
            HitId::LangAuto => self.set_language("auto"),
            HitId::LangItem(i) => {
                let code = self
                    .state
                    .i18n
                    .available()
                    .get(i as usize)
                    .map(|l| l.code.clone());
                if let Some(code) = code {
                    self.set_language(&code);
                }
            }
            _ => {}
        }
    }

    pub fn quality_track(&self) -> Option<DRect> {
        self.hits
            .iter()
            .find(|h| h.id == HitId::SaveQuality)
            .map(|h| h.rect)
    }

    pub fn set_quality_from_x(&mut self, x: f32) {
        if let Some(track) = self.quality_track() {
            let t = ((x - track.x) / track.w.max(1.0)).clamp(0.0, 1.0);
            self.state.save_quality = (1.0 + t * 99.0).round() as u8;
            self.invalidate();
        }
    }

    pub fn zoom_step(&mut self, factor: f32) {
        let anchor = if self.layout.canvas.contains(self.state.mouse) {
            self.state.mouse
        } else {
            self.layout.canvas.center()
        };
        self.zoom_to(self.state.view.zoom * factor, anchor);
    }

    pub fn zoom_to(&mut self, zoom: f32, anchor: crate::model::Point) {
        let old = self.state.view.zoom;
        let new = zoom.clamp(0.02, 8.0);
        if (old - new).abs() < 1e-5 {
            return;
        }
        let doc = crate::model::Point::new(
            (anchor.x - self.state.view.pan.x) / old,
            (anchor.y - self.state.view.pan.y) / old,
        );
        self.state.view.zoom = new;
        self.state.view.pan = crate::model::Point::new(anchor.x - doc.x * new, anchor.y - doc.y * new);
        self.state.view.fitted = false;
        self.state.clamp_pan(self.layout.canvas);
        self.invalidate();
    }

    pub fn fit_current(&mut self) {
        let padding = 48.0 * (self.dpi / 96.0);
        self.state.fit(self.layout.canvas, padding);
        self.invalidate();
    }

    pub fn about_text(&self) -> (String, String) {
        (
            self.state.i18n.t("about.intro_text").to_string(),
            self.state.i18n.t("about.tech_body").to_string(),
        )
    }

    pub fn popup_open(&self) -> bool {
        self.state.popup.is_some()
    }

    pub fn modal_open(&self) -> bool {
        self.state.modal.is_some()
    }

    pub fn unused(_: LPARAM, _: WPARAM, _: Rgba) {}
}
