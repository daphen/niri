use smithay::utils::{Logical, Rectangle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::layout) enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub(in crate::layout) fn nearest_contact<I: Clone + PartialEq>(
    current: &I,
    direction: Direction,
    items: &[(I, Rectangle<f64, Logical>, u64)],
    gap: f64,
) -> Option<I> {
    let (_, current_rect, _) = items.iter().find(|(id, _, _)| id == current)?;
    items
        .iter()
        .filter_map(|(id, rect, order)| {
            let (primary, perpendicular) = axis_gaps(*current_rect, *rect, direction)?;
            ((primary.into_inner() - gap).abs() < 0.001
                && perpendicular.into_inner() == 0.
                && perpendicular_overlap(*current_rect, *rect, direction) > 0.)
                .then_some((id, order))
        })
        .min_by_key(|(_, order)| *order)
        .map(|(id, _)| id.clone())
}

pub(in crate::layout) fn strict_contact(
    from: Rectangle<f64, Logical>,
    to: Rectangle<f64, Logical>,
    direction: Direction,
    gap: f64,
) -> bool {
    axis_gaps(from, to, direction).is_some_and(|(primary, perpendicular)| {
        (primary.into_inner() - gap).abs() < 0.001
            && perpendicular.into_inner() == 0.
            && perpendicular_overlap(from, to, direction) > 0.
    })
}

fn perpendicular_overlap(
    from: Rectangle<f64, Logical>,
    to: Rectangle<f64, Logical>,
    direction: Direction,
) -> f64 {
    let (a, a_size, b, b_size) = match direction {
        Direction::Left | Direction::Right => (from.loc.y, from.size.h, to.loc.y, to.size.h),
        Direction::Up | Direction::Down => (from.loc.x, from.size.w, to.loc.x, to.size.w),
    };
    (a + a_size).min(b + b_size) - a.max(b)
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
