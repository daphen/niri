use std::time::Duration;

use smithay::output::Output;
use smithay::utils::{Logical, Point};

use crate::input::swipe_tracker::SwipeTracker;
use crate::layout::plane::PlaneView;
use crate::layout::{Layout, LayoutElement};

#[derive(Debug, Default)]
pub(crate) struct PlaneGesture {
    active: Option<ActiveGesture>,
}

#[derive(Debug)]
enum ActiveGesture {
    Pan {
        output: Output,
        x: SwipeTracker,
        y: SwipeTracker,
        start: PlaneView,
        travel: f64,
    },
    Pinch {
        output: Output,
        centroid: Point<f64, Logical>,
        scale: f64,
        start: PlaneView,
    },
}

impl PlaneGesture {
    pub(crate) fn begin<W: LayoutElement>(&mut self, layout: &mut Layout<W>, output: Output) {
        self.begin_with_delta(layout, output, Point::default(), Duration::ZERO);
    }

    pub(crate) fn begin_with_delta<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        output: Output,
        initial_delta: Point<f64, Logical>,
        timestamp: Duration,
    ) -> Option<Output> {
        let start = layout.plane_pan_begin(&output)?;
        self.active = Some(ActiveGesture::Pan {
            output,
            x: SwipeTracker::new(),
            y: SwipeTracker::new(),
            start,
            travel: 0.,
        });
        self.update(layout, initial_delta, timestamp)
    }

    pub(crate) fn update<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        delta: Point<f64, Logical>,
        timestamp: Duration,
    ) -> Option<Output> {
        let ActiveGesture::Pan {
            output,
            x,
            y,
            start,
            travel,
        } = self.active.as_mut()?
        else {
            return None;
        };
        x.push(delta.x, timestamp);
        y.push(delta.y, timestamp);
        *travel += delta.x.hypot(delta.y);
        let zoom_progress = (*travel / 240.).clamp(0., 1.);
        layout.plane_pan_update(output, delta, *start, zoom_progress)
    }

    pub(crate) fn end<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        timestamp: Duration,
    ) -> Option<Output> {
        let ActiveGesture::Pan {
            output,
            mut x,
            mut y,
            start,
            ..
        } = self.active.take()?
        else {
            return None;
        };
        x.push(0., timestamp);
        y.push(0., timestamp);
        let projected_delta = Point::from((
            x.projected_end_pos() - x.pos(),
            y.projected_end_pos() - y.pos(),
        ));
        layout.plane_pan_end(&output, projected_delta, start)
    }

    pub(crate) fn pinch_begin<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        output: Output,
        centroid: Point<f64, Logical>,
    ) -> bool {
        let Some(start) = layout.plane_pinch_begin(&output) else {
            return false;
        };
        self.active = Some(ActiveGesture::Pinch {
            output,
            centroid,
            scale: 1.,
            start,
        });
        true
    }

    pub(crate) fn pinch_update<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        delta: Point<f64, Logical>,
        scale: f64,
    ) -> Option<Output> {
        let ActiveGesture::Pinch {
            output,
            centroid,
            scale: previous_scale,
            ..
        } = self.active.as_mut()?
        else {
            return None;
        };
        let relative_scale = scale / *previous_scale;
        *previous_scale = scale;
        let output = layout.plane_pinch_update(output, *centroid, delta, relative_scale);
        *centroid += delta;
        output
    }

    pub(crate) fn pinch_end<W: LayoutElement>(
        &mut self,
        layout: &mut Layout<W>,
        cancelled: bool,
    ) -> Option<Output> {
        let ActiveGesture::Pinch { output, start, .. } = self.active.take()? else {
            return None;
        };
        layout.plane_pinch_end(&output, cancelled.then_some(start))
    }
}
