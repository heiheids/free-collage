use std::time::{Duration, Instant};

use crate::codec::ExportFormat;
use crate::config::Settings;
use crate::i18n::I18n;
use crate::model::canvas::{CanvasMode, DEFAULT_DPI};
use crate::model::{Document, History, Point, Rect, Rgba, Style};
use crate::tools::{Cursor, Drag, Tool};
use crate::ui::HitId;

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub zoom: f32,
    pub pan: Point,
    pub fitted: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Point::ZERO,
            fitted: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CanvasForm {
    pub width: String,
    pub height: String,
    pub dpi: String,
    pub focus: u8,
    pub invalid: bool,
}

impl CanvasForm {
    pub fn from_mode(mode: &CanvasMode) -> Self {
        match mode {
            CanvasMode::Fixed {
                width_mm,
                height_mm,
                dpi,
            } => Self {
                width: trim_num(*width_mm),
                height: trim_num(*height_mm),
                dpi: trim_num(*dpi),
                focus: 0,
                invalid: false,
            },
            CanvasMode::Auto => Self {
                width: trim_num(crate::model::canvas::A4_MM.0),
                height: trim_num(crate::model::canvas::A4_MM.1),
                dpi: trim_num(DEFAULT_DPI),
                focus: 0,
                invalid: false,
            },
        }
    }

    pub fn parse(&self) -> Option<CanvasMode> {
        let width = self.width.trim().parse::<f32>().ok()?;
        let height = self.height.trim().parse::<f32>().ok()?;
        let dpi = self.dpi.trim().parse::<f32>().ok()?;
        if !(10.0..=2000.0).contains(&width)
            || !(10.0..=2000.0).contains(&height)
            || !(36.0..=1200.0).contains(&dpi)
        {
            return None;
        }
        Some(CanvasMode::clamp_fixed(width, height, dpi))
    }

    pub fn focus_next(&mut self, reverse: bool) {
        self.focus = if reverse {
            (self.focus + 2) % 3
        } else {
            (self.focus + 1) % 3
        };
    }

    pub fn active_text_mut(&mut self) -> &mut String {
        match self.focus {
            0 => &mut self.width,
            1 => &mut self.height,
            _ => &mut self.dpi,
        }
    }
}

fn trim_num(v: f32) -> String {
    if (v - v.round()).abs() < 0.01 {
        format!("{}", v.round() as i32)
    } else {
        format!("{v:.1}")
    }
}

#[derive(Clone, Debug)]
pub struct TextEditState {
    pub layer_id: Option<u64>,
    pub original: String,
    pub color: Rgba,
    pub font_size: f32,
    pub font_family: String,
    pub outline: Option<Rgba>,
    pub outline_width: f32,
    pub pos: Point,
    pub screen_rect: Rect,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub until: Instant,
}

#[derive(Clone, Debug)]
pub enum Popup {
    Color {
        target: crate::model::ColorTarget,
        anchor: Rect,
    },
    Canvas {
        anchor: Rect,
        custom: bool,
    },
    Language {
        anchor: Rect,
    },
    Font {
        anchor: Rect,
    },
    LayerMenu {
        anchor: Rect,
        id: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modal {
    Save,
    About,
    CloseConfirm,
}

pub struct State {
    pub doc: Document,
    pub history: History,
    pub selected: Option<u64>,
    pub tool: Tool,
    pub style: Style,
    pub view: View,
    pub i18n: I18n,
    pub lang_pref: String,
    pub popup: Option<Popup>,
    pub modal: Option<Modal>,
    pub drag: Drag,
    pub hover: Option<HitId>,
    pub mouse: Point,
    pub mouse_in_canvas: bool,
    pub text_edit: Option<TextEditState>,
    pub save_format: ExportFormat,
    pub save_quality: u8,
    pub save_transparent: bool,
    pub canvas_form: CanvasForm,
    pub toast: Option<Toast>,
    pub last_dir: String,
    pub cursor: Cursor,
    pub space_down: bool,
    pub is_maximized: bool,
    pub slider_drag: bool,
    pub anim: Instant,
    pub dirty: bool,
    pub close_after_save: bool,
}

impl State {
    pub fn new(settings: Settings, i18n: I18n) -> Self {
        let tool = Tool::Select;
        Self {
            doc: Document {
                canvas: settings.canvas.clone(),
                ..Document::new()
            },
            history: History::default(),
            selected: None,
            tool,
            style: settings.style.clone(),
            view: View::default(),
            i18n,
            lang_pref: settings.language.clone(),
            popup: None,
            modal: None,
            drag: Drag::None,
            hover: None,
            mouse: Point::ZERO,
            mouse_in_canvas: false,
            text_edit: None,
            save_format: ExportFormat::from_str(&settings.export_format),
            save_quality: settings.quality,
            save_transparent: settings.transparent,
            canvas_form: CanvasForm::from_mode(&settings.canvas),
            toast: None,
            last_dir: settings.last_dir.clone(),
            cursor: Cursor::Arrow,
            space_down: false,
            is_maximized: false,
            slider_drag: false,
            anim: Instant::now(),
            dirty: false,
            close_after_save: false,
        }
    }

    pub fn push_history(&mut self) {
        self.history.push(&self.doc);
        self.dirty = true;
    }

    pub fn undo(&mut self) -> bool {
        if let Some(doc) = self.history.pop() {
            self.doc = doc;
            self.dirty = true;
            if let Some(id) = self.selected {
                if self.doc.layer(id).is_none() {
                    self.selected = None;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn select(&mut self, id: Option<u64>) {
        self.selected = id;
    }

    pub fn selected_layer(&self) -> Option<&crate::model::Layer> {
        self.selected.and_then(|id| self.doc.layer(id))
    }

    pub fn selected_layer_mut(&mut self) -> Option<&mut crate::model::Layer> {
        self.selected.and_then(|id| self.doc.layer_mut(id))
    }

    pub fn selected_rect(&self) -> Option<Rect> {
        self.selected_layer().map(|l| l.rect)
    }

    pub fn page_rect(&self) -> Option<Rect> {
        self.doc.page_rect()
    }

    pub fn page_pixels(&self) -> Option<(u32, u32)> {
        match &self.doc.canvas {
            CanvasMode::Fixed { .. } => self.doc.canvas.pixel_size(),
            CanvasMode::Auto => self.doc.content_bounds().map(|r| {
                (
                    r.w.ceil().max(1.0) as u32,
                    r.h.ceil().max(1.0) as u32,
                )
            }),
        }
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            until: Instant::now() + Duration::from_millis(2600),
        });
    }

    pub fn toast_i18n(&mut self, key: &str) {
        let text = self.i18n.t(key).to_string();
        self.toast(text);
    }

    pub fn toast_fmt(&mut self, key: &str, args: &[&str]) {
        let text = self.i18n.fmt(key, args);
        self.toast(text);
    }

    pub fn clear_overlays(&mut self) {
        self.popup = None;
        self.modal = None;
        self.slider_drag = false;
    }

    pub fn close_popup(&mut self) {
        self.popup = None;
        self.slider_drag = false;
    }

    pub fn to_settings(&self) -> Settings {
        Settings {
            language: if self.lang_pref.is_empty() {
                "auto".into()
            } else {
                self.lang_pref.clone()
            },
            canvas: self.doc.canvas.clone(),
            style: self.style.clone(),
            export_format: self.save_format.as_str().into(),
            quality: self.save_quality,
            transparent: self.save_transparent,
            last_dir: self.last_dir.clone(),
        }
    }

    pub fn fit(&mut self, viewport: Rect, padding: f32) {
        let page = self.page_rect();
        let (zoom, pan) = crate::render::fit_view(page, viewport, padding);
        self.view.zoom = zoom;
        self.view.pan = pan;
        self.view.fitted = true;
    }

    pub fn clamp_pan(&mut self, viewport: Rect) {
        let Some(page) = self.page_rect() else {
            return;
        };
        if page.w <= 0.0 || page.h <= 0.0 {
            return;
        }
        let zoom = self.view.zoom.max(0.01);
        let screen = crate::render::page_screen_rect(page, zoom, self.view.pan);
        let margin = 96.0;
        let min_x = viewport.x + margin - screen.w;
        let max_x = viewport.right() - margin;
        let min_y = viewport.y + margin - screen.h;
        let max_y = viewport.bottom() - margin;
        if min_x <= max_x {
            self.view.pan.x = self.view.pan.x.clamp(min_x, max_x);
        }
        if min_y <= max_y {
            self.view.pan.y = self.view.pan.y.clamp(min_y, max_y);
        }
    }

    pub fn ensure_selection_visible(&mut self) {
        if let Some(id) = self.selected {
            if self.doc.layer(id).is_none() {
                self.selected = None;
            }
        }
    }
}

