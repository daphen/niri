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
