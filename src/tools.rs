#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Open,
    Paste,
    Delete,
    Clear,
    Undo,
    Save,
    Text,
    Brush,
    Mosaic,
    Eraser,
    Arrow,
    Border,
    Rect,
    Ellipse,
}

impl Tool {
    pub const ALL: [Tool; 15] = [
        Tool::Select,
        Tool::Open,
        Tool::Paste,
        Tool::Delete,
        Tool::Clear,
        Tool::Undo,
        Tool::Save,
        Tool::Text,
        Tool::Brush,
        Tool::Mosaic,
        Tool::Eraser,
        Tool::Arrow,
        Tool::Border,
        Tool::Rect,
        Tool::Ellipse,
    ];

    pub fn is_mode(self) -> bool {
        matches!(
            self,
            Tool::Select
                | Tool::Text
                | Tool::Brush
                | Tool::Mosaic
                | Tool::Eraser
                | Tool::Arrow
                | Tool::Border
                | Tool::Rect
                | Tool::Ellipse
        )
    }

    pub fn is_action(self) -> bool {
        !self.is_mode()
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Tool::Select => "tool.select",
            Tool::Open => "tool.open",
            Tool::Paste => "tool.paste",
            Tool::Delete => "tool.delete",
            Tool::Clear => "tool.clear",
            Tool::Undo => "tool.undo",
            Tool::Save => "tool.save",
            Tool::Text => "tool.text",
            Tool::Brush => "tool.brush",
            Tool::Mosaic => "tool.mosaic",
            Tool::Eraser => "tool.eraser",
            Tool::Arrow => "tool.arrow",
            Tool::Border => "tool.border",
            Tool::Rect => "tool.rect",
            Tool::Ellipse => "tool.ellipse",
        }
    }

    pub fn icon_name(self) -> &'static str {
        self.label_key()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub fn anchor(self, r: crate::model::Rect) -> crate::model::Point {
        match self {
            Corner::TopLeft => crate::model::Point::new(r.right(), r.bottom()),
            Corner::TopRight => crate::model::Point::new(r.x, r.bottom()),
            Corner::BottomLeft => crate::model::Point::new(r.right(), r.y),
            Corner::BottomRight => crate::model::Point::new(r.x, r.y),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Arrow,
    Hand,
    IBeam,
    Crosshair,
    SizeNS,
    SizeWE,
    SizeNWSE,
    SizeNESW,
    Move,
}

#[derive(Clone, Debug)]
pub enum Drag {
    None,
    Pan {
        start_mouse: crate::model::Point,
        start_pan: crate::model::Point,
    },
    Move {
        id: u64,
        start_mouse: crate::model::Point,
        start_layer: crate::model::Layer,
        pushed: bool,
    },
    Scale {
        id: u64,
        corner: Corner,
        start_mouse: crate::model::Point,
        start_layer: crate::model::Layer,
        pushed: bool,
    },
    ShapeCreate {
        kind: crate::model::ShapeKind,
        start: crate::model::Point,
        current: crate::model::Point,
    },
    Stroke {
        points: Vec<crate::model::Point>,
    },
    Mosaic {
        id: Option<u64>,
        last: crate::model::Point,
        pushed: bool,
    },
    Erase {
        last: crate::model::Point,
        pushed: bool,
    },
}

impl Drag {
    pub fn is_none(&self) -> bool {
        matches!(self, Drag::None)
    }
}
