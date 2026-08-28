use std::time::Duration;

use smithay::output::Output;
use smithay::utils::{Logical, Point};

use crate::input::swipe_tracker::SwipeTracker;
use crate::layout::{Layout, LayoutElement};

#[derive(Debug, Default)]
pub(crate) struct PlaneGesture {
    active: Option<ActiveGesture>,
}

#[derive(Debug)]
struct ActiveGesture {
    output: Output,
    x: SwipeTracker,
    y: SwipeTracker,
}

impl PlaneGesture {
    pub(crate) fn begin<W: LayoutElement>(&mut self, layout: &mut Layout<W>, output: Output) {
        layout.plane_pan_begin(&output);
        self.active = Some(ActiveGesture {
            output,
            x: SwipeTracker::new(),
            y: SwipeTracker::new(),
        });
    }

    pub(crate) fn update<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        delta: Point<f64, Logical>,
        timestamp: Duration,
    ) -> Option<Output> {
        let active = self.active.as_mut()?;
        active.x.push(delta.x, timestamp);
        active.y.push(delta.y, timestamp);
        layout.plane_pan_update(&active.output, delta)
    }

    pub(crate) fn end<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        timestamp: Duration,
    ) -> Option<Output> {
        let mut active = self.active.take()?;
        active.x.push(0., timestamp);
        active.y.push(0., timestamp);
        let projected_delta = Point::from((
            active.x.projected_end_pos() - active.x.pos(),
            active.y.projected_end_pos() - active.y.pos(),
        ));
        layout.plane_pan_end(&active.output, projected_delta)
    }
}
