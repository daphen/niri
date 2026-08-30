use smithay::utils::{Logical, Rectangle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::layout) enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub(in crate::layout) fn nearest<I: Clone + PartialEq>(
    current: &I,
    direction: Direction,
    items: &[(I, Rectangle<f64, Logical>, u64)],
) -> Option<I> {
    let (_, current_rect, _) = items.iter().find(|(id, _, _)| id == current)?;
    items
        .iter()
        .filter(|(id, rect, _)| {
            id != current && axis_gaps(*current_rect, *rect, direction).is_some()
        })
        .min_by(|a, b| {
            axis_gaps(*current_rect, a.1, direction)
                .unwrap()
                .partial_cmp(&axis_gaps(*current_rect, b.1, direction).unwrap())
                .unwrap()
                .then_with(|| a.2.cmp(&b.2))
        })
        .map(|(id, _, _)| id.clone())
}

fn axis_gaps(
    from: Rectangle<f64, Logical>,
    to: Rectangle<f64, Logical>,
    direction: Direction,
) -> Option<(ordered_float::NotNan<f64>, ordered_float::NotNan<f64>)> {
    let (primary, secondary) = match direction {
        Direction::Left => (
            from.loc.x - (to.loc.x + to.size.w),
            interval_gap(from.loc.y, from.size.h, to.loc.y, to.size.h),
        ),
        Direction::Right => (
            to.loc.x - (from.loc.x + from.size.w),
            interval_gap(from.loc.y, from.size.h, to.loc.y, to.size.h),
        ),
        Direction::Up => (
            from.loc.y - (to.loc.y + to.size.h),
            interval_gap(from.loc.x, from.size.w, to.loc.x, to.size.w),
        ),
        Direction::Down => (
            to.loc.y - (from.loc.y + from.size.h),
            interval_gap(from.loc.x, from.size.w, to.loc.x, to.size.w),
        ),
    };
    (primary >= 0.).then(|| {
        (
            ordered_float::NotNan::new(primary).unwrap(),
            ordered_float::NotNan::new(secondary).unwrap(),
        )
    })
}

fn interval_gap(a: f64, a_size: f64, b: f64, b_size: f64) -> f64 {
    (a - (b + b_size)).max(b - (a + a_size)).max(0.)
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Axis {
    X,
    Y,
}

pub(super) struct Body<I> {
    pub spatial_id: super::ColumnId,
    pub members: Vec<I>,
    pub rectangle: Rectangle<f64, Logical>,
}

pub(super) struct Snapshot<I> {
    pub bodies: Vec<Body<I>>,
    pub contacts: Vec<(usize, usize, Axis)>,
}

pub(super) fn nearest_body<I: PartialEq>(
    current: &I,
    direction: Direction,
    snapshot: &Snapshot<I>,
) -> Option<super::ColumnId> {
    if !connected(snapshot) {
        return None;
    }
    let current = snapshot
        .bodies
        .iter()
        .find(|body| body.members.contains(current))?;
    snapshot
        .bodies
        .iter()
        .filter(|body| body.spatial_id != current.spatial_id)
        .filter_map(|body| {
            metrics(current.rectangle, body.rectangle, direction).map(|metrics| (body, metrics))
        })
        .min_by(|(a, a_metrics), (b, b_metrics)| {
            compare_metrics(*a_metrics, *b_metrics)
                .then_with(|| compare_rectangle(a.rectangle, b.rectangle, direction))
                .then_with(|| a.spatial_id.get().cmp(&b.spatial_id.get()))
        })
        .map(|(body, _)| body.spatial_id)
}

fn connected<I>(snapshot: &Snapshot<I>) -> bool {
    if snapshot.bodies.len() < 2 {
        return true;
    }
    let mut reached = vec![0];
    let mut next = 0;
    while next < reached.len() {
        for &(a, b, axis) in &snapshot.contacts {
            if !matches!(axis, Axis::X | Axis::Y) {
                continue;
            }
            let candidate = if a == reached[next] {
                b
            } else if b == reached[next] {
                a
            } else {
                continue;
            };
            if !reached.contains(&candidate) {
                reached.push(candidate);
            }
        }
        next += 1;
    }
    reached.len() == snapshot.bodies.len()
}

fn metrics(
    from: Rectangle<f64, Logical>,
    to: Rectangle<f64, Logical>,
    direction: Direction,
) -> Option<(f64, f64, f64)> {
    let (extends, primary, perpendicular) = match direction {
        Direction::Left => (
            to.loc.x < from.loc.x,
            interval_gap(from.loc.x, from.size.w, to.loc.x, to.size.w),
            interval_gap(from.loc.y, from.size.h, to.loc.y, to.size.h),
        ),
        Direction::Right => (
            to.loc.x + to.size.w > from.loc.x + from.size.w,
            interval_gap(from.loc.x, from.size.w, to.loc.x, to.size.w),
            interval_gap(from.loc.y, from.size.h, to.loc.y, to.size.h),
        ),
        Direction::Up => (
            to.loc.y < from.loc.y,
            interval_gap(from.loc.y, from.size.h, to.loc.y, to.size.h),
            interval_gap(from.loc.x, from.size.w, to.loc.x, to.size.w),
        ),
        Direction::Down => (
            to.loc.y + to.size.h > from.loc.y + from.size.h,
            interval_gap(from.loc.y, from.size.h, to.loc.y, to.size.h),
            interval_gap(from.loc.x, from.size.w, to.loc.x, to.size.w),
        ),
    };
    extends.then_some((
        primary * primary + perpendicular * perpendicular,
        primary,
        perpendicular,
    ))
}

fn compare_metrics(a: (f64, f64, f64), b: (f64, f64, f64)) -> std::cmp::Ordering {
    a.0.total_cmp(&b.0)
        .then_with(|| a.1.total_cmp(&b.1))
        .then_with(|| a.2.total_cmp(&b.2))
}

fn compare_rectangle(
    a: Rectangle<f64, Logical>,
    b: Rectangle<f64, Logical>,
    direction: Direction,
) -> std::cmp::Ordering {
    let key = |rect: Rectangle<f64, Logical>| match direction {
        Direction::Right => [rect.loc.x, rect.loc.y, rect.size.w, rect.size.h],
        Direction::Left => [
            -(rect.loc.x + rect.size.w),
            rect.loc.y,
            rect.size.w,
            rect.size.h,
        ],
        Direction::Down => [rect.loc.y, rect.loc.x, rect.size.h, rect.size.w],
        Direction::Up => [
            -(rect.loc.y + rect.size.h),
            rect.loc.x,
            rect.size.h,
            rect.size.w,
        ],
    };
    let a = key(a);
    let b = key(b);
    a.into_iter()
        .zip(b)
        .find_map(|(a, b)| (a != b).then(|| a.total_cmp(&b)))
        .unwrap_or(std::cmp::Ordering::Equal)
}
