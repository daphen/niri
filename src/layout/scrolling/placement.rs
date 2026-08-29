use std::collections::{BTreeMap, HashSet};

use smithay::utils::{Logical, Point, Rectangle, Size};

use super::ColumnId;

#[derive(Debug, Clone)]
pub(super) struct Item {
    pub id: ColumnId,
    pub app_id: Option<String>,
    pub order: u64,
    pub size: Size<f64, Logical>,
    pub current: Option<Point<f64, Logical>>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Target {
    pub id: ColumnId,
    pub position: Point<f64, Logical>,
}

#[derive(Debug, Default)]
pub(super) struct State {
    anchors: BTreeMap<String, Point<f64, Logical>>,
    items: Vec<(ColumnId, u64, Size<f64, Logical>, Option<String>)>,
}

impl State {
    pub fn invalidate(&mut self) {
        self.items.clear();
    }

    pub fn remember(&mut self, items: &[Item]) {
        self.items = items
            .iter()
            .map(|item| (item.id, item.order, item.size, item.app_id.clone()))
            .collect();
    }

    pub fn needs_arrange(&self, items: &[Item]) -> bool {
        self.items.len() != items.len()
            || items.iter().any(|item| {
                !self.items.iter().any(|old| {
                    old.0 == item.id
                        && old.1 == item.order
                        && old.2 == item.size
                        && old.3 == item.app_id
                })
            })
    }

    pub fn arrange(&mut self, items: &[Item], gap: f64) -> Vec<Target> {
        let live_apps: HashSet<_> = items
            .iter()
            .filter_map(|item| item.app_id.as_ref())
            .cloned()
            .collect();
        self.anchors.retain(|app, _| live_apps.contains(app));

        let mut ordered: Vec<_> = items.iter().collect();
        ordered.sort_by_key(|item| item.order);
        let mut occupied = Vec::with_capacity(items.len());
        let mut targets = Vec::with_capacity(items.len());
        for item in ordered {
            let anchor = item
                .app_id
                .as_ref()
                .and_then(|app| self.anchors.get(app).copied())
                .or(item.current)
                .unwrap_or_default();
            let position = item
                .current
                .filter(|position| free(Rectangle::new(*position, item.size), gap, &occupied))
                .unwrap_or_else(|| nearest_free(anchor, item.size, gap, &occupied));
            if let Some(app) = &item.app_id {
                self.anchors.entry(app.clone()).or_insert(position);
            }
            occupied.push(Rectangle::new(position, item.size));
            targets.push(Target {
                id: item.id,
                position,
            });
        }
        self.items = items
            .iter()
            .map(|item| (item.id, item.order, item.size, item.app_id.clone()))
            .collect();
        targets
    }
}

fn nearest_free(
    anchor: Point<f64, Logical>,
    size: Size<f64, Logical>,
    gap: f64,
    occupied: &[Rectangle<f64, Logical>],
) -> Point<f64, Logical> {
    let mut xs = vec![anchor.x];
    let mut ys = vec![anchor.y];
    for rect in occupied {
        xs.extend([
            rect.loc.x - size.w - gap,
            rect.loc.x,
            rect.loc.x + rect.size.w - size.w,
            rect.loc.x + rect.size.w + gap,
        ]);
        ys.extend([
            rect.loc.y - size.h - gap,
            rect.loc.y,
            rect.loc.y + rect.size.h - size.h,
            rect.loc.y + rect.size.h + gap,
        ]);
    }
    xs.into_iter()
        .flat_map(|x| ys.iter().map(move |y| Point::from((x, *y))))
        .filter(|position| free(Rectangle::new(*position, size), gap, occupied))
        .min_by(|a, b| {
            distance(*a, anchor)
                .total_cmp(&distance(*b, anchor))
                .then_with(|| a.x.total_cmp(&b.x))
                .then_with(|| a.y.total_cmp(&b.y))
        })
        .unwrap()
}

fn distance(a: Point<f64, Logical>, b: Point<f64, Logical>) -> f64 {
    let delta = a - b;
    delta.x * delta.x + delta.y * delta.y
}

fn free(
    rectangle: Rectangle<f64, Logical>,
    gap: f64,
    occupied: &[Rectangle<f64, Logical>],
) -> bool {
    occupied.iter().all(|other| {
        rectangle.loc.x + rectangle.size.w + gap <= other.loc.x
            || other.loc.x + other.size.w + gap <= rectangle.loc.x
            || rectangle.loc.y + rectangle.size.h + gap <= other.loc.y
            || other.loc.y + other.size.h + gap <= rectangle.loc.y
    })
}
