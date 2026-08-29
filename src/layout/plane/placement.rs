use std::collections::{BTreeMap, HashSet};

use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::layout::scrolling::ColumnId;

#[derive(Debug, Clone)]
pub(in crate::layout) struct Item {
    pub id: ColumnId,
    pub app_id: Option<String>,
    pub order: u64,
    pub size: Size<f64, Logical>,
    pub current: Option<Point<f64, Logical>>,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::layout) struct Target {
    pub id: ColumnId,
    pub position: Point<f64, Logical>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum GroupKey {
    App(String),
    Window(u64),
}

#[derive(Debug, Default)]
pub(in crate::layout) struct State {
    anchors: BTreeMap<String, Point<f64, Logical>>,
    groups: Vec<(ColumnId, u64)>,
}

impl State {
    pub fn needs_arrange(&self, items: &[Item]) -> bool {
        self.groups.len() != items.len()
            || items.iter().any(|item| {
                !self
                    .groups
                    .iter()
                    .any(|group| group.0 == item.id && group.1 == item.order)
            })
    }

    pub fn invalidate(&mut self) {
        self.groups.clear();
    }

    pub fn sync_anchor(&mut self, item: &Item, items: &[Item], position: Point<f64, Logical>) {
        let Some(app_id) = &item.app_id else {
            return;
        };
        if items
            .iter()
            .filter(|other| other.app_id.as_ref() == Some(app_id))
            .all(|other| item.order <= other.order)
        {
            self.anchors.insert(app_id.clone(), position);
        }
    }

    pub fn arrange(&mut self, items: &[Item], gap: f64) -> Vec<Target> {
        let mut grouped = BTreeMap::<GroupKey, Vec<&Item>>::new();
        for item in items {
            let key = item
                .app_id
                .clone()
                .map(GroupKey::App)
                .unwrap_or(GroupKey::Window(item.order));
            grouped.entry(key).or_default().push(item);
        }
        let mut groups: Vec<_> = grouped.into_values().collect();
        for group in &mut groups {
            group.sort_by_key(|item| item.order);
        }
        groups.sort_by_key(|group| group[0].order);

        let live_apps: HashSet<_> = groups
            .iter()
            .filter_map(|group| group[0].app_id.as_ref())
            .cloned()
            .collect();
        self.anchors.retain(|app, _| live_apps.contains(app));

        let mut occupied = Vec::<Rectangle<f64, Logical>>::new();
        for group in &groups {
            let anchor = group[0];
            if let Some(app_id) = &anchor.app_id {
                let position = self
                    .anchors
                    .get(app_id)
                    .copied()
                    .or(anchor.current)
                    .filter(|position| free(Rectangle::new(*position, anchor.size), &occupied))
                    .unwrap_or_else(|| {
                        nearest_free(Point::default(), anchor.size, anchor.size, gap, &occupied)
                    });
                self.anchors.insert(app_id.clone(), position);
                occupied.push(Rectangle::new(position, anchor.size));
            }
        }
        for group in &groups {
            occupied.extend(group.iter().skip(1).filter_map(|item| {
                item.current
                    .map(|position| Rectangle::new(position, item.size))
            }));
        }

        let mut targets = Vec::with_capacity(items.len());
        for group in &groups {
            for item in group.iter().skip(1) {
                if let Some(position) = item.current {
                    let rectangle = Rectangle::new(position, item.size);
                    occupied.retain(|other| *other != rectangle);
                }
            }

            let anchor = group[0];
            let anchor_position = anchor
                .app_id
                .as_ref()
                .and_then(|app| self.anchors.get(app).copied())
                .or_else(|| {
                    anchor
                        .current
                        .filter(|position| free(Rectangle::new(*position, anchor.size), &occupied))
                })
                .unwrap_or_else(|| {
                    nearest_free(Point::default(), anchor.size, anchor.size, gap, &occupied)
                });
            if anchor.app_id.is_none() {
                occupied.push(Rectangle::new(anchor_position, anchor.size));
            }
            targets.push(Target {
                id: anchor.id,
                position: anchor_position,
            });

            for item in group.iter().skip(1) {
                let position =
                    nearest_free(anchor_position, item.size, anchor.size, gap, &occupied);
                occupied.push(Rectangle::new(position, item.size));
                targets.push(Target {
                    id: item.id,
                    position,
                });
            }
        }
        self.groups = items.iter().map(|item| (item.id, item.order)).collect();
        targets
    }
}

fn nearest_free(
    anchor: Point<f64, Logical>,
    size: Size<f64, Logical>,
    anchor_size: Size<f64, Logical>,
    gap: f64,
    occupied: &[Rectangle<f64, Logical>],
) -> Point<f64, Logical> {
    let step: Size<f64, Logical> = Size::from((
        anchor_size.w.max(size.w) + gap,
        anchor_size.h.max(size.h) + gap,
    ));
    for ring in 0.. {
        let mut points = Vec::new();
        for x in -ring..=ring {
            points.push((x, -ring));
            points.push((x, ring));
        }
        for y in (-ring + 1)..ring {
            points.push((-ring, y));
            points.push((ring, y));
        }
        points.sort_by(|a, b| {
            let distance =
                |(x, y): &(i32, i32)| (*x as f64 * step.w).powi(2) + (*y as f64 * step.h).powi(2);
            distance(a).total_cmp(&distance(b)).then_with(|| a.cmp(b))
        });
        for (x, y) in points {
            let position = anchor + Point::from((x as f64 * step.w, y as f64 * step.h));
            if free(Rectangle::new(position, size), occupied) {
                return position;
            }
        }
    }
    unreachable!()
}

fn free(rectangle: Rectangle<f64, Logical>, occupied: &[Rectangle<f64, Logical>]) -> bool {
    occupied.iter().all(|other| !overlaps(rectangle, *other))
}

fn overlaps(a: Rectangle<f64, Logical>, b: Rectangle<f64, Logical>) -> bool {
    a.loc.x < b.loc.x + b.size.w
        && a.loc.x + a.size.w > b.loc.x
        && a.loc.y < b.loc.y + b.size.h
        && a.loc.y + a.size.h > b.loc.y
}
