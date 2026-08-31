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

        if let Some(changed) = items.iter().find(|item| {
            self.items
                .iter()
                .any(|old| old.0 == item.id && old.2 != item.size)
        }) {
            let position = changed.current.unwrap_or_default();
            let resized = Rectangle::new(position, changed.size);
            let old_size = self.items.iter().find(|old| old.0 == changed.id).unwrap().2;
            if let Some(targets) = preserve_resize_contacts(items, changed, old_size, gap) {
                self.remember(items);
                return targets;
            }
            let reserved: Vec<_> = items
                .iter()
                .filter(|item| item.id != changed.id)
                .filter_map(|item| Some((item.id, Rectangle::new(item.current?, item.size))))
                .collect();
            let targets = items
                .iter()
                .map(|item| {
                    let current = item.current.unwrap_or_default();
                    let position = if item.id == changed.id
                        || free(Rectangle::new(current, item.size), 0., &[resized])
                    {
                        current
                    } else {
                        let occupied: Vec<_> = reserved
                            .iter()
                            .filter_map(|(id, rect)| (id != &item.id).then_some(*rect))
                            .chain([resized])
                            .collect();
                        let right = Point::from((resized.loc.x + resized.size.w + gap, current.y));
                        let left = Point::from((resized.loc.x - item.size.w - gap, current.y));
                        let down = Point::from((current.x, resized.loc.y + resized.size.h + gap));
                        let up = Point::from((current.x, resized.loc.y - item.size.h - gap));
                        let contacts = if old_size.w != changed.size.w {
                            [right, down, up, left]
                        } else {
                            [down, up, right, left]
                        };
                        contacts
                            .into_iter()
                            .find(|point| free(Rectangle::new(*point, item.size), 0., &occupied))
                            .unwrap_or_else(|| nearest_free(current, item.size, gap, &occupied))
                    };
                    let id = item.id;
                    Target { id, position }
                })
                .collect();
            self.remember(items);
            return targets;
        }

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

fn preserve_resize_contacts(
    items: &[Item],
    changed: &Item,
    old_size: Size<f64, Logical>,
    gap: f64,
) -> Option<Vec<Target>> {
    let old = Rectangle::new(changed.current?, old_size);
    let delta: Point<f64, Logical> =
        Point::from((changed.size.w - old_size.w, changed.size.h - old_size.h));
    let others = items
        .iter()
        .filter(|item| item.id != changed.id)
        .collect::<Vec<_>>();
    let rectangles = others
        .iter()
        .map(|item| Rectangle::new(item.current.unwrap(), item.size))
        .collect::<Vec<_>>();
    let contact = |a: Rectangle<f64, Logical>, b: Rectangle<f64, Logical>| {
        let overlap_x = (a.loc.x + a.size.w).min(b.loc.x + b.size.w) - a.loc.x.max(b.loc.x);
        let overlap_y = (a.loc.y + a.size.h).min(b.loc.y + b.size.h) - a.loc.y.max(b.loc.y);
        (overlap_y > 0.
            && ((b.loc.x - a.loc.x - a.size.w - gap).abs() < 0.001
                || (a.loc.x - b.loc.x - b.size.w - gap).abs() < 0.001))
            || (overlap_x > 0.
                && ((b.loc.y - a.loc.y - a.size.h - gap).abs() < 0.001
                    || (a.loc.y - b.loc.y - b.size.h - gap).abs() < 0.001))
    };
    let mut shifts = vec![Point::default(); others.len()];
    let mut affected = false;
    for seed in 0..others.len() {
        let right = delta.x != 0.
            && (rectangles[seed].loc.x - old.loc.x - old.size.w - gap).abs() < 0.001
            && (old.loc.y + old.size.h).min(rectangles[seed].loc.y + rectangles[seed].size.h)
                > old.loc.y.max(rectangles[seed].loc.y);
        let down = delta.y != 0.
            && (rectangles[seed].loc.y - old.loc.y - old.size.h - gap).abs() < 0.001
            && (old.loc.x + old.size.w).min(rectangles[seed].loc.x + rectangles[seed].size.w)
                > old.loc.x.max(rectangles[seed].loc.x);
        if !right && !down {
            continue;
        }
        affected = true;
        let mut component = vec![seed];
        let mut next = 0;
        while next < component.len() {
            for idx in 0..others.len() {
                if !component.contains(&idx)
                    && contact(rectangles[component[next]], rectangles[idx])
                {
                    component.push(idx);
                }
            }
            next += 1;
        }
        for idx in component {
            if right {
                shifts[idx].x = delta.x;
            }
            if down {
                shifts[idx].y = delta.y;
            }
        }
    }
    if !affected {
        return None;
    }
    let targets = items
        .iter()
        .map(|item| {
            let shift = others
                .iter()
                .position(|other| other.id == item.id)
                .map(|idx| shifts[idx])
                .unwrap_or_default();
            Target {
                id: item.id,
                position: item.current.unwrap() + shift,
            }
        })
        .collect::<Vec<_>>();
    let placed = items
        .iter()
        .map(|item| {
            let target = targets.iter().find(|target| target.id == item.id).unwrap();
            Rectangle::new(target.position, item.size)
        })
        .collect::<Vec<_>>();
    placed
        .iter()
        .enumerate()
        .all(|(idx, rectangle)| free(*rectangle, gap, &placed[idx + 1..]))
        .then_some(targets)
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

pub(super) fn place_new(
    preferred: Point<f64, Logical>,
    size: Size<f64, Logical>,
    gap: f64,
    occupied: &[Rectangle<f64, Logical>],
) -> Point<f64, Logical> {
    if free(Rectangle::new(preferred, size), gap, occupied) {
        return preferred;
    }
    occupied
        .iter()
        .flat_map(|other| {
            [
                Point::from((other.loc.x + other.size.w + gap, other.loc.y)),
                Point::from((other.loc.x - size.w - gap, other.loc.y)),
                Point::from((other.loc.x, other.loc.y + other.size.h + gap)),
                Point::from((other.loc.x, other.loc.y - size.h - gap)),
            ]
        })
        .filter(|position| free(Rectangle::new(*position, size), gap, occupied))
        .min_by(|a, b| distance(*a, preferred).total_cmp(&distance(*b, preferred)))
        .unwrap()
}
