use crate::codec::ExportFormat;
use crate::model::style::PALETTE;
use crate::model::{ColorTarget, Rect};
use crate::render::icons::{draw_icon, draw_logo, fill_round_rect, stroke_round_rect, Icon};
use crate::render::theme;
use crate::render::{color_target_label, Align, Painter};
use crate::state::{Modal, Popup};
use crate::ui::HitId;

impl<'a> Painter<'a> {
    pub(crate) fn paint_popup(&mut self) {
        let Some(popup) = self.state.popup.clone() else {
            return;
        };
        match popup {
            Popup::Color { target, anchor } => self.popup_color(target, anchor),
            Popup::Canvas { anchor, custom } => self.popup_canvas(anchor, custom),
            Popup::Language { anchor } => self.popup_language(anchor),
            Popup::Font { anchor } => self.popup_font(anchor),
            Popup::LayerMenu { anchor, id } => self.popup_layer_menu(anchor, id),
        }
    }

    fn place_popup(&self, anchor: Rect, w: f32, h: f32, above: bool) -> Rect {
        let l = &self.layout;
        let margin = l.px(8.0);
        let x = anchor.x.min(l.width - w - margin).max(margin);
        let mut y = if above {
            anchor.y - h - l.px(8.0)
        } else {
            anchor.bottom() + l.px(8.0)
        };
        if y + h > l.height - margin {
            y = anchor.y - h - l.px(8.0);
        }
        if y < margin {
            y = margin;
        }
        Rect::new(x, y, w, h)
    }

    fn popup_panel(&mut self, rect: Rect) {
        let l = self.layout.clone();
        // soft shadow
        fill_round_rect(
            &mut self.pixmap,
            Rect::new(rect.x + l.px(2.0), rect.y + l.px(5.0), rect.w, rect.h),
            l.px(14.0),
            theme::SHADOW,
        );
        self.round(rect, l.px(14.0), theme::PANEL);
        self.outline(rect, l.px(14.0), theme::LINE, 1.0);
    }

    fn popup_color(&mut self, target: ColorTarget, anchor: Rect) {
        let l = self.layout.clone();
        let w = l.px(300.0);
        let h = l.px(228.0);
        let rect = self.place_popup(anchor, w, h, true);
        self.popup_panel(rect);
        let title = color_target_label(self.state, target);
        self.text_at(rect.x + l.px(16.0), rect.y + l.px(14.0), &title, l.px(13.5), 600, theme::INK);

        let cols = 8;
        let sw = l.px(26.0);
        let gap = l.px(6.0);
        let start_x = rect.x + l.px(16.0);
        let start_y = rect.y + l.px(42.0);
        let current = match target {
            ColorTarget::Border => Some(self.state.style.border),
            ColorTarget::Fill => self.state.style.fill,
            ColorTarget::Text => Some(self.state.style.text),
            ColorTarget::TextOutline => self.state.style.text_outline,
        };
        for (i, color) in PALETTE.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let r = Rect::new(
                start_x + col as f32 * (sw + gap),
                start_y + row as f32 * (sw + gap),
                sw,
                sw,
            );
            if self.hover(HitId::Swatch(i as u8)) {
                self.round(r.inflate(l.px(2.0)), l.px(7.0), theme::HOVER);
            }
            fill_round_rect(&mut self.pixmap, r, l.px(6.0), theme::WHITE);
            self.outline(r, l.px(6.0), theme::LINE, 1.0);
            fill_round_rect(&mut self.pixmap, r.inflate(-l.px(2.0)), l.px(5.0), *color);
            if current == Some(*color) {
                stroke_round_rect(&mut self.pixmap, r.inflate(l.px(1.0)), l.px(7.0), theme::BLUE, l.px(2.0));
            }
            self.push_hit(HitId::Swatch(i as u8), r);
        }

        let mut y = start_y + 3.0 * (sw + gap) + l.px(4.0);
        if target == ColorTarget::Fill || target == ColorTarget::TextOutline {
            let none = Rect::new(rect.x + l.px(16.0), y, l.px(126.0), l.px(30.0));
            if self.hover(HitId::ColorNone) {
                self.round(none, l.px(7.0), theme::HOVER);
            }
            self.outline(none, l.px(7.0), theme::LINE, 1.0);
            let label = self.t("color.none");
            self.text_in(none, &label, l.px(11.0), 500, theme::INK, Align::Center);
            self.push_hit(HitId::ColorNone, none);
            let custom = Rect::new(none.right() + l.px(10.0), y, rect.right() - none.right() - l.px(26.0), l.px(30.0));
            self.secondary_popup_button(custom, HitId::ColorCustom, "color.custom");
            y += l.px(32.0);
        } else {
            let custom = Rect::new(rect.x + l.px(16.0), y, rect.w - l.px(32.0), l.px(30.0));
            self.secondary_popup_button(custom, HitId::ColorCustom, "color.custom");
            y += l.px(32.0);
        }
        let _ = y;
    }

    fn popup_canvas(&mut self, anchor: Rect, custom: bool) {
        let l = self.layout.clone();
        let w = l.px(320.0);
        let base_h = l.px(260.0);
        let form_h = if custom { l.px(178.0) } else { 0.0 };
        let h = base_h + form_h;
        let rect = self.place_popup(anchor, w, h, false);
        self.popup_panel(rect);
        let title = self.t("canvas.canvas_title");
        self.text_at(rect.x + l.px(16.0), rect.y + l.px(14.0), &title, l.px(13.5), 600, theme::INK);

        let current = &self.state.doc.canvas;
        let is_auto = current.is_auto();
        let is_a4 = matches!(current, crate::model::CanvasMode::Fixed { width_mm, height_mm, .. } if (*width_mm - 210.0).abs() < 0.6 && (*height_mm - 297.0).abs() < 0.6);
        let is_a4l = matches!(current, crate::model::CanvasMode::Fixed { width_mm, height_mm, .. } if (*width_mm - 297.0).abs() < 0.6 && (*height_mm - 210.0).abs() < 0.6);
        let is_b5 = matches!(current, crate::model::CanvasMode::Fixed { width_mm, height_mm, .. } if (*width_mm - 176.0).abs() < 0.6 && (*height_mm - 250.0).abs() < 0.6);
        let is_b5l = matches!(current, crate::model::CanvasMode::Fixed { width_mm, height_mm, .. } if (*width_mm - 250.0).abs() < 0.6 && (*height_mm - 176.0).abs() < 0.6);
        let is_custom = !is_auto && !is_a4 && !is_a4l && !is_b5 && !is_b5l;
        let rows = [
            (HitId::CanvasAuto, "canvas.auto", is_auto),
            (HitId::CanvasA4, "canvas.a4", is_a4),
            (HitId::CanvasA4Landscape, "canvas.a4_landscape", is_a4l),
            (HitId::CanvasB5, "canvas.b5", is_b5),
            (HitId::CanvasB5Landscape, "canvas.b5_landscape", is_b5l),
            (HitId::CanvasCustom, "canvas.custom", is_custom || custom),
        ];
        let mut y = rect.y + l.px(40.0);
        for (id, key, selected) in rows {
            let r = Rect::new(rect.x + l.px(10.0), y, rect.w - l.px(20.0), l.px(34.0));
            if selected {
                self.round(r, l.px(7.0), theme::BLUE_SOFT);
            } else if self.hover(id) {
                self.round(r, l.px(7.0), theme::HOVER);
            }
            let label = self.t(key);
            self.text_in(
                Rect::new(r.x + l.px(10.0), r.y, r.w - l.px(34.0), r.h),
                &label,
                l.px(12.0),
                500,
                if selected { theme::BLUE } else { theme::INK },
                Align::Left,
            );
            if selected {
                draw_icon(
                    &mut self.pixmap,
                    Icon::Check,
                    Rect::new(r.right() - l.px(26.0), r.center().y - l.px(8.0), l.px(16.0), l.px(16.0)),
                    theme::BLUE,
                    l.px(2.0),
                );
            }
            self.push_hit(id, r);
            y += l.px(35.0);
        }

        if custom {
            let form = self.state.canvas_form.clone();
            let labels = [("canvas.width", form.width), ("canvas.height", form.height), ("canvas.dpi", form.dpi)];
            let mut fy = y + l.px(4.0);
            for (i, (key, value)) in labels.iter().enumerate() {
                let label = self.t(key);
                self.text_in(
                    Rect::new(rect.x + l.px(18.0), fy, l.px(120.0), l.px(30.0)),
                    &label,
                    l.px(10.5),
                    400,
                    theme::MUTED,
                    Align::Left,
                );
                let field = Rect::new(rect.x + l.px(142.0), fy, rect.w - l.px(160.0), l.px(30.0));
                let focused = form.focus == i as u8;
                self.round(field, l.px(7.0), theme::PANEL_ALT);
                self.outline(field, l.px(7.0), if focused { theme::BLUE } else { theme::LINE }, if focused { 1.6 } else { 1.0 });
                self.text_in(
                    field.inflate(-l.px(10.0)),
                    value,
                    l.px(12.0),
                    500,
                    theme::INK,
                    Align::Left,
                );
                if focused {
                    let (tw, _) = self.text.measure(value, l.px(12.0), 500);
                    self.fill(
                        Rect::new(field.x + l.px(10.0) + tw + l.px(1.0), field.center().y - l.px(7.0), l.px(1.2), l.px(14.0)),
                        theme::BLUE,
                    );
                }
                self.push_hit(HitId::CanvasField(i as u8), field);
                fy += l.px(36.0);
            }
            let apply = Rect::new(rect.right() - l.px(100.0), fy + l.px(2.0), l.px(84.0), l.px(30.0));
            self.primary_button(apply, HitId::CanvasApply, "canvas.apply", None);
            if form.invalid {
                self.text_in(
                    Rect::new(rect.x + l.px(18.0), fy + l.px(6.0), rect.w - l.px(130.0), l.px(22.0)),
                    "10–2000 mm / 36–1200 DPI",
                    l.px(8.5),
                    400,
                    theme::DANGER,
                    Align::Left,
                );
            }
            let hint = self.t("canvas.hint");
            let _ = hint;
        }
    }

    fn popup_language(&mut self, anchor: Rect) {
        let l = self.layout.clone();
        let langs: Vec<(String, String)> = self
            .state
            .i18n
            .available()
            .iter()
            .map(|x| (x.code.clone(), x.name.clone()))
            .collect();
        let n = langs.len() + 1;
        let w = l.px(196.0);
        let h = l.px(20.0) + n as f32 * l.px(34.0) + l.px(10.0);
        let rect = self.place_popup(anchor, w, h, false);
        self.popup_panel(rect);
        let mut y = rect.y + l.px(10.0);
        let rows: Vec<(HitId, String, bool)> = std::iter::once((
            HitId::LangAuto,
            self.t("lang.system"),
            self.state.lang_pref == "auto",
        ))
        .chain(langs.iter().enumerate().map(|(i, (code, name))| {
            (HitId::LangItem(i as u8), name.clone(), self.state.lang_pref == *code)
        }))
        .collect();
        for (id, label, selected) in rows {
            let r = Rect::new(rect.x + l.px(8.0), y, rect.w - l.px(16.0), l.px(32.0));
            if selected {
                self.round(r, l.px(7.0), theme::BLUE_SOFT);
            } else if self.hover(id) {
                self.round(r, l.px(7.0), theme::HOVER);
            }
            self.text_in(
                Rect::new(r.x + l.px(10.0), r.y, r.w - l.px(36.0), r.h),
                &label,
                l.px(11.5),
                500,
                if selected { theme::BLUE } else { theme::INK },
                Align::Left,
            );
            if selected {
                draw_icon(
                    &mut self.pixmap,
                    Icon::Check,
                    Rect::new(r.right() - l.px(26.0), r.center().y - l.px(8.0), l.px(16.0), l.px(16.0)),
                    theme::BLUE,
                    l.px(2.0),
                );
            }
            self.push_hit(id, r);
            y += l.px(34.0);
        }
    }

    fn secondary_popup_button(&mut self, rect: Rect, id: HitId, key: &str) {
        let l = self.layout.clone();
        let hover = self.hover(id);
        self.round(rect, l.px(7.0), if hover { theme::HOVER } else { theme::PANEL_ALT });
        self.outline(rect, l.px(7.0), theme::LINE, 1.0);
        let label = self.t(key);
        self.text_in(rect, &label, l.px(11.0), 500, theme::INK, Align::Center);
        self.push_hit(id, rect);
    }

    pub(crate) fn paint_modal(&mut self) {
        let Some(modal) = self.state.modal else {
            return;
        };
        let l = self.layout.clone();
        self.fill(Rect::new(0.0, 0.0, l.width, l.height), theme::MODAL_DIM);
        match modal {
            Modal::Save => self.modal_save(),
            Modal::About => self.modal_about(),
            Modal::CloseConfirm => self.modal_close_confirm(),
            Modal::CropConfirm => self.modal_crop_confirm(),
        }
    }

    fn modal_save(&mut self) {
        let l = self.layout.clone();
        let panel = Rect::new(l.width * 0.5 - l.px(300.0), l.height * 0.5 - l.px(225.0), l.px(600.0), l.px(450.0));
        self.modal_panel(panel);
        let title = self.t("save.title");
        self.text_at(panel.x + l.px(28.0), panel.y + l.px(22.0), &title, l.px(20.0), 600, theme::INK);
        let size = self.state.page_pixels();
        let size_text = if let Some((w, h)) = size {
            self.state.i18n.fmt("save.summary", &[&w.to_string(), &h.to_string()])
        } else {
            self.t("status.nothing")
        };
        self.text_at(panel.x + l.px(28.0), panel.y + l.px(56.0), &size_text, l.px(11.0), 400, theme::MUTED);

        let format_label = self.t("save.format");
        self.text_at(panel.x + l.px(28.0), panel.y + l.px(92.0), &format_label, l.px(11.0), 600, theme::MUTED);
        let seg_w = (panel.w - l.px(56.0) - l.px(8.0) * 3.0) / 4.0;
        for (i, format) in ExportFormat::ALL.iter().enumerate() {
            let r = Rect::new(panel.x + l.px(28.0) + i as f32 * (seg_w + l.px(8.0)), panel.y + l.px(112.0), seg_w, l.px(40.0));
            let selected = self.state.save_format == *format;
            self.round(r, l.px(8.0), if selected { theme::BLUE } else if self.hover(HitId::SaveFormat(i as u8)) { theme::HOVER } else { theme::PANEL_ALT });
            self.outline(r, l.px(8.0), if selected { theme::BLUE } else { theme::LINE }, 1.0);
            self.text_in(r, format.label(), l.px(12.0), 600, if selected { theme::WHITE } else { theme::INK }, Align::Center);
            self.push_hit(HitId::SaveFormat(i as u8), r);
        }

        let qy = panel.y + l.px(176.0);
        let quality = self.t("save.quality");
        self.text_at(panel.x + l.px(28.0), qy, &quality, l.px(11.0), 600, theme::MUTED);
        let value = if self.state.save_format.quality_enabled() {
            format!("{}", self.state.save_quality)
        } else {
            self.t("save.quality_lossless")
        };
        let (vw, _) = self.text.measure(&value, l.px(11.5), 500);
        self.text_at(panel.right() - l.px(28.0) - vw, qy - l.px(1.0), &value, l.px(11.5), 500, theme::INK);

        let track = Rect::new(panel.x + l.px(28.0), qy + l.px(26.0), panel.w - l.px(56.0), l.px(6.0));
        let enabled = self.state.save_format.quality_enabled();
        self.round(track, l.px(3.0), if enabled { theme::LINE } else { theme::LINE.with_alpha(120) });
        if enabled {
            let t = (self.state.save_quality as f32 - 1.0) / 99.0;
            let knob_x = track.x + track.w * t.clamp(0.0, 1.0);
            self.round(Rect::new(track.x, track.y, (knob_x - track.x).max(0.0), track.h), l.px(3.0), theme::BLUE);
            fill_round_rect(&mut self.pixmap, Rect::new(knob_x - l.px(9.0), track.center().y - l.px(9.0), l.px(18.0), l.px(18.0)), l.px(9.0), theme::WHITE);
            stroke_round_rect(&mut self.pixmap, Rect::new(knob_x - l.px(9.0), track.center().y - l.px(9.0), l.px(18.0), l.px(18.0)), l.px(9.0), theme::BLUE, l.px(2.0));
            self.push_hit(HitId::SaveQuality, track.inflate(l.px(12.0)));
        }
        let qhint = self.t("save.quality_hint");
        self.text_at(panel.x + l.px(28.0), qy + l.px(44.0), &qhint, l.px(10.0), 400, theme::FAINT);

        let ty = panel.y + l.px(268.0);
        let transparent = self.state.save_transparent && self.state.save_format.supports_alpha();
        let box_rect = Rect::new(panel.x + l.px(28.0), ty, l.px(20.0), l.px(20.0));
        self.round(box_rect, l.px(4.0), if transparent { theme::BLUE } else { theme::PANEL });
        self.outline(box_rect, l.px(4.0), if transparent { theme::BLUE } else { theme::LINE_STRONG }, 1.0);
        if transparent {
            draw_icon(
                &mut self.pixmap,
                Icon::Check,
                box_rect.inflate(-l.px(3.0)),
                theme::WHITE,
                l.px(2.0),
            );
        }
        let tlabel = self.t("save.transparent");
        self.text_in(
            Rect::new(box_rect.right() + l.px(10.0), ty - l.px(3.0), panel.w - l.px(100.0), l.px(24.0)),
            &tlabel,
            l.px(12.0),
            500,
            theme::INK,
            Align::Left,
        );
        self.push_hit(HitId::SaveTransparent, Rect::new(panel.x + l.px(22.0), ty - l.px(7.0), panel.w - l.px(44.0), l.px(34.0)));
        let thint = self.t("save.transparent_hint");
        self.text_at(panel.x + l.px(28.0), ty + l.px(30.0), &thint, l.px(9.5), 400, theme::FAINT);

        let by = panel.bottom() - l.px(66.0);
        let export = Rect::new(panel.right() - l.px(174.0), by, l.px(146.0), l.px(42.0));
        let cancel = Rect::new(export.x - l.px(120.0), by, l.px(110.0), l.px(42.0));
        self.secondary_button(cancel, HitId::SaveCancel, "save.cancel", None);
        if self.state.page_rect().is_some() {
            self.primary_button(export, HitId::SaveExport, "save.export", None);
        } else {
            self.round(export, l.px(8.0), theme::LINE);
            let label = self.t("save.export");
            self.text_in(export, &label, l.px(12.0), 600, theme::FAINT, Align::Center);
        }
    }

    fn modal_close_confirm(&mut self) {
        let l = self.layout.clone();
        let panel = Rect::new(
            l.width * 0.5 - l.px(210.0),
            l.height * 0.5 - l.px(105.0),
            l.px(420.0),
            l.px(210.0),
        );
        self.modal_panel(panel);
        let title = self.t("close.title");
        self.text_at(panel.x + l.px(28.0), panel.y + l.px(22.0), &title, l.px(19.0), 600, theme::INK);
        let message = self.t("close.message");
        self.wrapped(
            Rect::new(panel.x + l.px(28.0), panel.y + l.px(60.0), panel.w - l.px(56.0), l.px(62.0)),
            &message,
            l.px(11.5),
            400,
            theme::MUTED,
            Align::Left,
        );

        let by = panel.bottom() - l.px(60.0);
        let cancel = Rect::new(panel.right() - l.px(28.0) - l.px(90.0), by, l.px(90.0), l.px(40.0));
        let discard = Rect::new(cancel.x - l.px(10.0) - l.px(110.0), by, l.px(110.0), l.px(40.0));
        let save = Rect::new(discard.x - l.px(10.0) - l.px(110.0), by, l.px(110.0), l.px(40.0));
        self.primary_button(save, HitId::CloseSave, "close.save", None);
        self.secondary_button(discard, HitId::CloseDiscard, "close.discard", None);
        self.secondary_button(cancel, HitId::CloseCancel, "close.cancel", None);
    }

    fn modal_crop_confirm(&mut self) {
        let l = self.layout.clone();
        let panel = Rect::new(
            l.width * 0.5 - l.px(210.0),
            l.height * 0.5 - l.px(110.0),
            l.px(420.0),
            l.px(220.0),
        );
        self.modal_panel(panel);
        let title = self.t("crop.title");
        self.text_at(
            panel.x + l.px(28.0),
            panel.y + l.px(22.0),
            &title,
            l.px(19.0),
            600,
            theme::INK,
        );
        let message = self.t("crop.message");
        self.wrapped(
            Rect::new(
                panel.x + l.px(28.0),
                panel.y + l.px(60.0),
                panel.w - l.px(56.0),
                l.px(62.0),
            ),
            &message,
            l.px(11.5),
            400,
            theme::MUTED,
            Align::Left,
        );
        let by = panel.bottom() - l.px(60.0);
        let cancel = Rect::new(
            panel.right() - l.px(28.0) - l.px(90.0),
            by,
            l.px(90.0),
            l.px(40.0),
        );
        let apply = Rect::new(
            cancel.x - l.px(10.0) - l.px(110.0),
            by,
            l.px(110.0),
            l.px(40.0),
        );
        self.primary_button(apply, HitId::CropApply, "crop.apply", None);
        self.secondary_button(cancel, HitId::CropCancel, "crop.cancel", None);
    }

    fn modal_about(&mut self) {
        let l = self.layout.clone();
        let panel = Rect::new(l.width * 0.5 - l.px(300.0), l.height * 0.5 - l.px(240.0), l.px(600.0), l.px(480.0));
        self.modal_panel(panel);
        let logo = Rect::new(panel.x + l.px(30.0), panel.y + l.px(28.0), l.px(60.0), l.px(60.0));
        draw_logo(&mut self.pixmap, logo);
        let title = self.t("about.title");
        self.text_at(logo.right() + l.px(18.0), panel.y + l.px(28.0), &title, l.px(21.0), 600, theme::INK);
        let version = self.state.i18n.fmt("about.version", &[crate::platform::system::version()]);
        self.text_at(logo.right() + l.px(18.0), panel.y + l.px(60.0), &version, l.px(11.0), 400, theme::MUTED);

        let mut y = panel.y + l.px(112.0);
        let author_label = self.t("about.author");
        self.text_at(panel.x + l.px(30.0), y, &author_label, l.px(10.5), 400, theme::MUTED);
        self.text_at(panel.x + l.px(104.0), y - l.px(1.0), "黑山大叔", l.px(12.0), 600, theme::INK);
        y += l.px(28.0);
        let home_label = self.t("about.homepage");
        self.text_at(panel.x + l.px(30.0), y, &home_label, l.px(10.5), 400, theme::MUTED);
        let home = "https://space.bilibili.com/19428259";
        self.text_at(panel.x + l.px(104.0), y, home, l.px(11.5), 500, theme::BLUE);
        let (hw, _) = self.text.measure(home, l.px(11.5), 500);
        self.fill(Rect::new(panel.x + l.px(104.0), y + l.px(18.0), hw, 1.0), theme::BLUE.with_alpha(120));
        self.push_hit(HitId::Homepage, Rect::new(panel.x + l.px(100.0), y - l.px(4.0), hw + l.px(12.0), l.px(26.0)));

        y = panel.y + l.px(184.0);
        let intro_label = self.t("about.intro");
        self.text_at(panel.x + l.px(30.0), y, &intro_label, l.px(11.0), 600, theme::MUTED);
        y += l.px(24.0);
        let intro = self.t("about.intro_text");
        self.wrapped(
            Rect::new(panel.x + l.px(30.0), y, panel.w - l.px(60.0), l.px(88.0)),
            &intro,
            l.px(12.0),
            400,
            theme::INK,
            Align::Left,
        );
        y = panel.y + l.px(316.0);
        let tech_label = self.t("about.tech");
        self.text_at(panel.x + l.px(30.0), y, &tech_label, l.px(11.0), 600, theme::MUTED);
        y += l.px(24.0);
        let tech = self.t("about.tech_body");
        self.wrapped(
            Rect::new(panel.x + l.px(30.0), y, panel.w - l.px(60.0), l.px(60.0)),
            &tech,
            l.px(10.5),
            400,
            theme::MUTED,
            Align::Left,
        );

        let close = Rect::new(panel.right() - l.px(150.0), panel.bottom() - l.px(66.0), l.px(120.0), l.px(42.0));
        self.primary_button(close, HitId::AboutClose, "about.close", None);
        let site = self.t("tooltip.homepage");
        self.text_at(panel.x + l.px(30.0), panel.bottom() - l.px(44.0), &site, l.px(9.5), 400, theme::FAINT);
    }

    fn popup_font(&mut self, anchor: Rect) {
        let l = self.layout.clone();
        let w = l.px(260.0);
        let row_h = l.px(30.0);
        let h = l.px(34.0) + row_h * crate::model::style::FONT_CHOICES.len() as f32 + l.px(52.0);
        let rect = self.place_popup(anchor, w, h, true);
        self.popup_panel(rect);
        let title = self.t("settings.font_family");
        self.text_at(rect.x + l.px(14.0), rect.y + l.px(12.0), &title, l.px(12.0), 600, theme::MUTED);

        let mut y = rect.y + l.px(36.0);
        for (i, (id, label)) in crate::model::style::FONT_CHOICES.iter().enumerate() {
            let r = Rect::new(rect.x + l.px(8.0), y, rect.w - l.px(16.0), row_h);
            let selected = self.state.style.font_family == *id;
            if selected {
                self.round(r, l.px(6.0), theme::BLUE_SOFT);
            } else if self.hover(HitId::FontChoice(i as u8)) {
                self.round(r, l.px(6.0), theme::HOVER);
            }
            self.text_in(
                Rect::new(r.x + l.px(8.0), r.y, r.w - l.px(30.0), r.h),
                label,
                l.px(10.5),
                if selected { 600 } else { 400 },
                if selected { theme::BLUE } else { theme::INK },
                Align::Left,
            );
            if selected {
                draw_icon(
                    &mut self.pixmap,
                    Icon::Check,
                    Rect::new(r.right() - l.px(24.0), r.center().y - l.px(7.0), l.px(14.0), l.px(14.0)),
                    theme::BLUE,
                    l.px(1.8),
                );
            }
            self.push_hit(HitId::FontChoice(i as u8), r);
            y += row_h;
        }

        let edge = Rect::new(rect.x + l.px(8.0), y + l.px(4.0), rect.w - l.px(16.0), l.px(32.0));
        if self.hover(HitId::TextOutlineColor) {
            self.round(edge, l.px(6.0), theme::HOVER);
        }
        let label = self.t("settings.text_outline");
        self.text_in(
            Rect::new(edge.x + l.px(8.0), edge.y, edge.w - l.px(48.0), edge.h),
            &label,
            l.px(10.5),
            500,
            theme::INK,
            Align::Left,
        );
        let sw = Rect::new(edge.right() - l.px(28.0), edge.center().y - l.px(9.0), l.px(18.0), l.px(18.0));
        if let Some(color) = self.state.style.text_outline {
            self.round(sw, l.px(4.0), color);
            self.outline(sw, l.px(4.0), theme::LINE_STRONG, 1.0);
        } else {
            self.round(sw, l.px(4.0), theme::CHECKER_B);
            self.round(Rect::new(sw.x, sw.y, sw.w * 0.5, sw.h * 0.5), l.px(3.0), theme::CHECKER_A);
            self.outline(sw, l.px(4.0), theme::LINE_STRONG, 1.0);
        }
        self.push_hit(HitId::TextOutlineColor, edge);
    }

    fn popup_layer_menu(&mut self, anchor: Rect, id: u64) {
        let l = self.layout.clone();
        let w = l.px(160.0);
        let h = l.px(82.0);
        let rect = self.place_popup(anchor, w, h, false);
        self.popup_panel(rect);
        let Some(index) = self.state.doc.layers.iter().position(|layer| layer.id == id) else {
            return;
        };
        let len = self.state.doc.layers.len();
        let rows = [
            (HitId::LayerUp, "layer.move_up", index + 1 < len),
            (HitId::LayerDown, "layer.move_down", index > 0),
        ];
        for (i, (hit, key, enabled)) in rows.into_iter().enumerate() {
            let r = Rect::new(rect.x + l.px(6.0), rect.y + l.px(6.0) + i as f32 * l.px(35.0), rect.w - l.px(12.0), l.px(32.0));
            if enabled && self.hover(hit) {
                self.round(r, l.px(6.0), theme::HOVER);
            }
            let label = self.t(key);
            self.text_in(
                r,
                &label,
                l.px(10.5),
                500,
                if enabled { theme::INK } else { theme::FAINT },
                Align::Center,
            );
            if enabled {
                self.push_hit(hit, r);
            }
        }
    }

    fn modal_panel(&mut self, panel: Rect) {
        let l = self.layout.clone();
        fill_round_rect(
            &mut self.pixmap,
            Rect::new(panel.x + l.px(4.0), panel.y + l.px(10.0), panel.w, panel.h),
            l.px(20.0),
            theme::MODAL_SHADOW,
        );
        self.round(panel, l.px(20.0), theme::PANEL);
        self.outline(panel, l.px(20.0), theme::LINE, 1.0);
    }
}
