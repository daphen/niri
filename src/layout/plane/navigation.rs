use smithay::utils::{Logical, Point, Rectangle};

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
    let current_center = center(*current_rect);
    items
        .iter()
        .filter(|(id, rect, _)| id != current && in_cone(current_center, center(*rect), direction))
        .min_by(|a, b| {
            score(current_center, center(a.1), direction)
                .total_cmp(&score(current_center, center(b.1), direction))
                .then_with(|| a.2.cmp(&b.2))
        })
        .map(|(id, _, _)| id.clone())
}

pub(in crate::layout) fn swap_positions(
    current: Rectangle<f64, Logical>,
    target: Rectangle<f64, Logical>,
    direction: Direction,
    occupied: &[Rectangle<f64, Logical>],
    gap: f64,
) -> (Point<f64, Logical>, Point<f64, Logical>) {
    let current_at_target = Rectangle::new(target.loc, current.size);
    let target_at_current = Rectangle::new(current.loc, target.size);
    if !overlaps(current_at_target, target_at_current)
        && occupied
            .iter()
            .all(|rect| !overlaps(current_at_target, *rect) && !overlaps(target_at_current, *rect))
    {
        return (target.loc, current.loc);
    }

    let current_position = free_position(current_at_target, direction, occupied, gap);
    let mut with_current = occupied.to_vec();
    with_current.push(Rectangle::new(current_position, current.size));
    let target_position = free_position(target_at_current, opposite(direction), &with_current, gap);
    (current_position, target_position)
}

pub(in crate::layout) fn free_position(
    rect: Rectangle<f64, Logical>,
    direction: Direction,
    occupied: &[Rectangle<f64, Logical>],
    gap: f64,
) -> Point<f64, Logical> {
    let step = match direction {
        Direction::Left => Point::from((-(rect.size.w + gap), 0.)),
        Direction::Right => Point::from((rect.size.w + gap, 0.)),
        Direction::Up => Point::from((0., -(rect.size.h + gap))),
        Direction::Down => Point::from((0., rect.size.h + gap)),
    };
    for distance in 1..=100_000 {
        let position = rect.loc + step.upscale(distance as f64);
        let candidate = Rectangle::new(position, rect.size);
        if occupied.iter().all(|other| !overlaps(candidate, *other)) {
            return position;
        }
    }
    rect.loc
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
    }
}

fn center(rect: Rectangle<f64, Logical>) -> Point<f64, Logical> {
    rect.loc + rect.size.to_point().downscale(2.)
}

fn in_cone(from: Point<f64, Logical>, to: Point<f64, Logical>, direction: Direction) -> bool {
    let delta = to - from;
    match direction {
        Direction::Left => delta.x < 0. && delta.x.abs() >= delta.y.abs(),
        Direction::Right => delta.x > 0. && delta.x.abs() >= delta.y.abs(),
        Direction::Up => delta.y < 0. && delta.y.abs() >= delta.x.abs(),
        Direction::Down => delta.y > 0. && delta.y.abs() >= delta.x.abs(),
    }
}

fn score(from: Point<f64, Logical>, to: Point<f64, Logical>, direction: Direction) -> f64 {
    let delta = to - from;
    let (primary, secondary) = match direction {
        Direction::Left | Direction::Right => (delta.x.abs(), delta.y.abs()),
        Direction::Up | Direction::Down => (delta.y.abs(), delta.x.abs()),
    };
    primary * primary + secondary * secondary
}

fn overlaps(a: Rectangle<f64, Logical>, b: Rectangle<f64, Logical>) -> bool {
    a.loc.x < b.loc.x + b.size.w
        && a.loc.x + a.size.w > b.loc.x
        && a.loc.y < b.loc.y + b.size.h
        && a.loc.y + a.size.h > b.loc.y
}
