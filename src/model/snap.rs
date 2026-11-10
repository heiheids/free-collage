use super::geometry::Rect;

/// Find a translation that aligns the moving rectangle's left/center/right
/// and top/center/bottom edges with nearby target edges.
///
/// X and Y are selected independently, so a horizontal and a vertical edge can
/// snap at the same time. `threshold` is in document units (screen pixels are
/// converted by the caller using the current zoom).
pub fn offset(rect: Rect, targets: &[Rect], page: Option<Rect>, threshold: f32) -> (f32, f32) {
    if threshold <= 0.0 {
        return (0.0, 0.0);
    }
    let mut xs = Vec::with_capacity(targets.len() * 3 + 3);
    let mut ys = Vec::with_capacity(targets.len() * 3 + 3);
    for target in targets {
        if target.w <= 0.0 || target.h <= 0.0 {
            continue;
        }
        let center = target.center();
        xs.extend([target.x, center.x, target.right()]);
        ys.extend([target.y, center.y, target.bottom()]);
    }
    if let Some(page) = page {
        if page.w > 0.0 && page.h > 0.0 {
            let center = page.center();
            xs.extend([page.x, center.x, page.right()]);
            ys.extend([page.y, center.y, page.bottom()]);
        }
    }

    let rect_center = rect.center();
    let dx = nearest(
        [rect.x, rect_center.x, rect.right()],
        &xs,
        threshold,
    )
    .unwrap_or(0.0);
    let dy = nearest(
        [rect.y, rect_center.y, rect.bottom()],
        &ys,
        threshold,
    )
    .unwrap_or(0.0);
    (dx, dy)
}

fn nearest(edges: [f32; 3], targets: &[f32], threshold: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for edge in edges {
        for target in targets {
            let delta = *target - edge;
            if delta.abs() <= threshold
                && best.map(|value| delta.abs() < value.abs()).unwrap_or(true)
            {
                best = Some(delta);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_horizontal_and_vertical_edges_together() {
        let moving = Rect::new(100.0, 100.0, 50.0, 50.0);
        let target = Rect::new(151.0, 149.0, 40.0, 40.0);
        assert_eq!(offset(moving, &[target], None, 2.0), (1.0, -1.0));
    }

    #[test]
    fn ignores_targets_outside_threshold() {
        let moving = Rect::new(100.0, 100.0, 50.0, 50.0);
        let far = Rect::new(300.0, 300.0, 50.0, 50.0);
        assert_eq!(offset(moving, &[far], None, 8.0), (0.0, 0.0));
    }

    #[test]
    fn snaps_to_page_edges_and_center() {
        let moving = Rect::new(118.0, 140.0, 20.0, 20.0);
        let page = Rect::new(0.0, 0.0, 100.0, 200.0);
        // Right edge 138 -> page right 100 is too far; center 150 -> page
        // center 100 is too far, so no X snap. Bottom 160 -> page bottom 200
        // is too far. This should not move.
        assert_eq!(offset(moving, &[], Some(page), 8.0), (0.0, 0.0));

        let near = Rect::new(72.0, 172.0, 20.0, 20.0);
        // Right 92 -> page right 100 is +8; bottom 192 -> page bottom 200 is +8.
        assert_eq!(offset(near, &[], Some(page), 8.0), (8.0, 8.0));
    }
}
