use super::canvas::CanvasMode;
use super::geometry::{Point, Rect};
use super::layer::{Layer, LayerKind, MosaicCell, ShapeKind};
use super::style::Rgba;

#[derive(Clone, Debug)]
pub struct Document {
    pub canvas: CanvasMode,
    pub layers: Vec<Layer>,
    pub next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Self {
            canvas: CanvasMode::Auto,
            layers: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add(&mut self, mut layer: Layer) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        layer.id = id;
        self.layers.push(layer);
        id
    }

    pub fn layer(&self, id: u64) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_mut(&mut self, id: u64) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.layers.len();
        self.layers.retain(|l| l.id != id);
        self.layers.len() != before
    }

    pub fn move_layer(&mut self, id: u64, up: bool) -> bool {
        let Some(index) = self.layers.iter().position(|layer| layer.id == id) else {
            return false;
        };
        let target = if up {
            if index + 1 >= self.layers.len() {
                return false;
            }
            index + 1
        } else {
            if index == 0 {
                return false;
            }
            index - 1
        };
        self.layers.swap(index, target);
        true
    }

    pub fn clear(&mut self) {
        self.layers.clear();
    }

    pub fn hit_test(&self, p: Point, tolerance: f32) -> Option<u64> {
        self.layers
            .iter()
            .rev()
            .find(|l| l.hit_test(p, tolerance))
            .map(|l| l.id)
    }

    pub fn hit_test_image(&self, p: Point, tolerance: f32) -> Option<u64> {
        self.layers
            .iter()
            .rev()
            .find(|l| l.is_image() && l.hit_test(p, tolerance))
            .map(|l| l.id)
    }

    pub fn sample(&self, p: Point) -> Option<Rgba> {
        self.layers.iter().rev().find_map(|l| l.sample(p))
    }

    pub fn content_bounds(&self) -> Option<Rect> {
        self.layers
            .iter()
            .filter(|l| l.visible && !l.rect.is_empty())
            .fold(None, |acc, l| match acc {
                None => Some(l.rect),
                Some(r) => Some(r.union(l.rect)),
            })
    }

    pub fn page_rect(&self) -> Option<Rect> {
        if let Some((w, h)) = self.canvas.pixel_size() {
            Some(Rect::new(0.0, 0.0, w as f32, h as f32))
        } else {
            self.content_bounds()
        }
    }

    pub fn image_count(&self) -> usize {
        self.layers.iter().filter(|l| l.is_image()).count()
    }

    pub fn cycle_selection(&self, current: Option<u64>, backwards: bool) -> Option<u64> {
        if self.layers.is_empty() {
            return None;
        }
        let n = self.layers.len();
        let idx = current
            .and_then(|id| self.layers.iter().position(|l| l.id == id))
            .unwrap_or(if backwards { 0 } else { n - 1 });
        let next = if backwards {
            (idx + n - 1) % n
        } else {
            (idx + 1) % n
        };
        Some(self.layers[next].id)
    }

    pub fn add_mosaic_cell(&mut self, id: u64, cell: MosaicCell) {
        if let Some(layer) = self.layer_mut(id) {
            if let LayerKind::Mosaic { cells, .. } = &mut layer.kind {
                if !cells.iter().any(|c| {
                    (c.rect.x - cell.rect.x).abs() < 0.01 && (c.rect.y - cell.rect.y).abs() < 0.01
                }) {
                    cells.push(cell);
                    layer.rect = layer.rect.union(cell.rect);
                }
            }
        }
    }

    pub fn add_text(
        &mut self,
        text: String,
        color: Rgba,
        font_size: f32,
        rect: Rect,
    ) -> u64 {
        let layer = Layer::text(0, text, color, font_size, rect);
        self.add(layer)
    }

    pub fn add_shape(
        &mut self,
        kind: ShapeKind,
        a: Point,
        b: Point,
        stroke: Rgba,
        fill: Option<Rgba>,
        width: f32,
        radius: f32,
    ) -> u64 {
        let layer = Layer::shape(0, kind, a, b, stroke, fill, width, radius);
        self.add(layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_bounds_union() {
        let mut doc = Document::new();
        doc.add(Layer::shape(
            0,
            ShapeKind::Rect,
            Point::new(10.0, 10.0),
            Point::new(50.0, 40.0),
            Rgba::BLACK,
            None,
            2.0,
            0.0,
        ));
        doc.add(Layer::shape(
            0,
            ShapeKind::Rect,
            Point::new(30.0, 20.0),
            Point::new(80.0, 90.0),
            Rgba::BLACK,
            None,
            2.0,
            0.0,
        ));
        let b = doc.content_bounds().unwrap();
        assert_eq!((b.x, b.y, b.right(), b.bottom()), (10.0, 10.0, 80.0, 90.0));
    }

    #[test]
    fn move_layer_keeps_order_within_bounds() {
        let mut doc = Document::new();
        let a = doc.add(Layer::text(
            0,
            "a",
            Rgba::BLACK,
            20.0,
            Rect::new(0.0, 0.0, 50.0, 30.0),
        ));
        let b = doc.add(Layer::text(
            0,
            "b",
            Rgba::BLACK,
            20.0,
            Rect::new(10.0, 10.0, 50.0, 30.0),
        ));
        let c = doc.add(Layer::text(
            0,
            "c",
            Rgba::BLACK,
            20.0,
            Rect::new(20.0, 20.0, 50.0, 30.0),
        ));
        assert!(doc.move_layer(a, true));
        assert_eq!(doc.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![b, a, c]);
        assert!(doc.move_layer(c, false));
        assert_eq!(doc.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![b, c, a]);
        assert!(!doc.move_layer(a, true));
        assert!(doc.move_layer(a, false));
        assert_eq!(doc.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![b, a, c]);
    }
}
