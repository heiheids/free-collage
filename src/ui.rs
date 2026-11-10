use crate::model::Rect;
use crate::tools::Tool;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitId {
    Tool(Tool),
    AppIcon,
    Title,
    About,
    Language,
    Min,
    Max,
    Close,
    CanvasChip,
    Screenshot,
    BorderColor,
    FillColor,
    TextColor,
    FontFamily,
    FontChoice(u8),
    TextOutlineColor,
    LayerUp,
    LayerDown,
    FontMinus,
    FontPlus,
    StrokeMinus,
    StrokePlus,
    BrushMinus,
    BrushPlus,
    MosaicMinus,
    MosaicPlus,
    ZoomOut,
    ZoomIn,
    ZoomFit,
    Swatch(u8),
    ColorNone,
    ColorCustom,
    CanvasAuto,
    CanvasA4,
    CanvasA4Landscape,
    CanvasB5,
    CanvasB5Landscape,
    CanvasCustom,
    CanvasField(u8),
    CanvasApply,
    LangAuto,
    LangItem(u8),
    SaveFormat(u8),
    SaveQuality,
    SaveTransparent,
    SaveExport,
    SaveCancel,
    CloseSave,
    CloseDiscard,
    CloseCancel,
    AboutClose,
    Homepage,
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub id: HitId,
    pub rect: Rect,
}

impl Hit {
    pub fn new(id: HitId, rect: Rect) -> Self {
        Self { id, rect }
    }
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub scale: f32,
    pub large: bool,
    pub ui_boost: f32,
    pub width: f32,
    pub height: f32,
    pub header: Rect,
    pub toolbar: Rect,
    pub canvas: Rect,
    pub bottom: Rect,
    pub logo: Rect,
    pub title: Rect,
    pub app_name: Rect,
    pub about: Rect,
    pub lang: Rect,
    pub screenshot: Rect,
    pub min: Rect,
    pub max: Rect,
    pub close: Rect,
    pub canvas_chip: Rect,
    pub border_color: Rect,
    pub fill_color: Rect,
    pub text_color: Rect,
    pub font_minus: Rect,
    pub font_value: Rect,
    pub font_plus: Rect,
    pub font_family: Rect,
    pub stroke_minus: Rect,
    pub stroke_value: Rect,
    pub stroke_plus: Rect,
    pub brush_minus: Rect,
    pub brush_value: Rect,
    pub brush_plus: Rect,
    pub mosaic_minus: Rect,
    pub mosaic_value: Rect,
    pub mosaic_plus: Rect,
    pub zoom_out: Rect,
    pub zoom_value: Rect,
    pub zoom_in: Rect,
    pub zoom_fit: Rect,
    pub actual_size: Rect,
    pub tools: Vec<(Tool, Rect)>,
}

impl Layout {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        let s = |v: f32| v * scale;
        let large = width >= 1400.0 && height >= 980.0;
        let ui_boost = if large { 1.5 } else { 1.0 };
        let header_h = s(if large { 60.0 } else { 54.0 });
        let bottom_h = s(if large { 170.0 } else { 132.0 });
        let margin = s(12.0);
        let header = Rect::new(0.0, 0.0, width, header_h);
        let bottom = Rect::new(margin, height - bottom_h, (width - margin * 2.0).max(0.0), bottom_h);
        let toolbar = Rect::new(
            margin,
            header_h + s(10.0),
            s(190.0),
            (bottom.y - s(10.0) - header_h - s(10.0)).max(0.0),
        );
        let canvas = Rect::new(
            toolbar.right() + s(12.0),
            header_h + s(10.0),
            (width - toolbar.right() - s(24.0)).max(0.0),
            (bottom.y - s(10.0) - header_h - s(10.0)).max(0.0),
        );

        let logo = Rect::new(s(16.0), (header_h - s(30.0)) * 0.5, s(30.0), s(30.0));
        let app_name = Rect::new(s(56.0), (header_h - s(32.0)) * 0.5, s(190.0), s(32.0));
        let title_h = if large { s(34.0) } else { s(24.0) };
        let title = Rect::new(s(56.0), (header_h - title_h) * 0.5, s(190.0), title_h);
        let chip_h = if large { s(40.0) } else { s(34.0) };
        let canvas_chip = Rect::new(
            s(258.0),
            (header_h - chip_h) * 0.5,
            s(if large { 360.0 } else { 110.0 }),
            chip_h,
        );

        let close = Rect::new(width - s(50.0), 0.0, s(50.0), header_h);
        let max = Rect::new(width - s(100.0), 0.0, s(50.0), header_h);
        let min = Rect::new(width - s(150.0), 0.0, s(50.0), header_h);
        let side_h = if large { s(38.0) } else { s(30.0) };
        let side_y = (header_h - side_h) * 0.5;
        let about = Rect::new(width - s(202.0), side_y, s(36.0), side_h);
        let lang = Rect::new(width - s(306.0), side_y, s(100.0), side_h);
        let shot_w = if large { s(88.0) } else { s(64.0) };
        let screenshot = Rect::new(lang.x - s(10.0) - shot_w, side_y, shot_w, side_h);

        let bx = bottom.x + s(18.0);
        let chip_h = if large { s(42.0) } else { s(34.0) };
        let row1 = bottom.y + if large { s(46.0) } else { s(34.0) };
        let border_color = Rect::new(bx, row1, s(166.0), chip_h);
        let fill_color = Rect::new(border_color.right() + s(12.0), row1, s(138.0), chip_h);
        let text_color = Rect::new(fill_color.right() + s(26.0), row1, s(134.0), chip_h);
        let font_minus = Rect::new(text_color.right() + s(18.0), row1, s(30.0), chip_h);
        let font_value = Rect::new(font_minus.right(), row1, s(50.0), chip_h);
        let font_plus = Rect::new(font_value.right(), row1, s(30.0), chip_h);
        let font_family = Rect::new(font_plus.right() + s(16.0), row1, s(132.0), chip_h);

        let ctrl_h = if large { s(38.0) } else { s(30.0) };
        let row2 = bottom.y + if large { s(118.0) } else { s(86.0) };
        let stroke_minus = Rect::new(bx, row2, s(30.0), ctrl_h);
        let stroke_value = Rect::new(stroke_minus.right(), row2, s(44.0), ctrl_h);
        let stroke_plus = Rect::new(stroke_value.right(), row2, s(30.0), ctrl_h);
        let brush_minus = Rect::new(stroke_plus.right() + s(28.0), row2, s(30.0), ctrl_h);
        let brush_value = Rect::new(brush_minus.right(), row2, s(44.0), ctrl_h);
        let brush_plus = Rect::new(brush_value.right(), row2, s(30.0), ctrl_h);
        let mosaic_minus = Rect::new(brush_plus.right() + s(28.0), row2, s(30.0), ctrl_h);
        let mosaic_value = Rect::new(mosaic_minus.right(), row2, s(44.0), ctrl_h);
        let mosaic_plus = Rect::new(mosaic_value.right(), row2, s(30.0), ctrl_h);

        let zoom_fit = Rect::new(
            bottom.right() - s(if large { 64.0 } else { 56.0 }),
            row2,
            s(if large { 60.0 } else { 50.0 }),
            ctrl_h,
        );
        let zoom_in = Rect::new(zoom_fit.x - s(if large { 36.0 } else { 32.0 }), row2, s(30.0), ctrl_h);
        let zoom_value = Rect::new(
            zoom_in.x - s(if large { 78.0 } else { 58.0 }),
            row2,
            s(if large { 72.0 } else { 56.0 }),
            ctrl_h,
        );
        let zoom_out = Rect::new(zoom_value.x - s(34.0), row2, s(30.0), ctrl_h);
        let actual_size = Rect::new(
            zoom_out.x - s(if large { 200.0 } else { 176.0 }),
            row2,
            s(if large { 188.0 } else { 164.0 }),
            ctrl_h,
        );

        let mut tools = Vec::new();
        let inner = toolbar.inflate(-s(8.0));
        let gap = s(6.0);
        let col_gap = s(8.0);
        let rows = 8.0;
        let bw = (inner.w - col_gap) * 0.5;
        let avail = (inner.h - s(8.0)).max(0.0);
        let bh = ((avail - gap * (rows - 1.0)) / rows).clamp(s(32.0), s(92.0));
        let total = bh * rows + gap * (rows - 1.0);
        let start_y = inner.y + ((inner.h - total) * 0.5).max(0.0);
        for (i, tool) in Tool::ALL.iter().enumerate() {
            let col = if i < 7 { 0.0 } else { 1.0 };
            let row = if i < 7 { i as f32 } else { (i - 7) as f32 };
            let x = inner.x + col * (bw + col_gap);
            let y = start_y + row * (bh + gap);
            tools.push((*tool, Rect::new(x, y, bw, bh)));
        }

        Self {
            scale,
            large,
            ui_boost,
            width,
            height,
            header,
            toolbar,
            canvas,
            bottom,
            logo,
            title,
            app_name,
            about,
            lang,
            screenshot,
            min,
            max,
            close,
            canvas_chip,
            border_color,
            fill_color,
            text_color,
            font_minus,
            font_value,
            font_plus,
            font_family,
            stroke_minus,
            stroke_value,
            stroke_plus,
            brush_minus,
            brush_value,
            brush_plus,
            mosaic_minus,
            mosaic_value,
            mosaic_plus,
            zoom_out,
            zoom_value,
            zoom_in,
            zoom_fit,
            actual_size,
            tools,
        }
    }

    pub fn px(&self, v: f32) -> f32 {
        v * self.scale
    }
}
