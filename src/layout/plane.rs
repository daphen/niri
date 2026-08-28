use niri_config::Animation as AnimationConfig;
use smithay::utils::{Logical, Point, Rectangle, Size};

use crate::animation::{Animation, Clock};

#[derive(Debug)]
pub(super) struct Plane {
    position: Point<f64, Logical>,
    scale: f64,
    bounds: PlaneBounds,
    animation: Option<PlaneAnimation>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlaneTransform {
    position: Point<f64, Logical>,
    scale: f64,
}

#[derive(Debug, Clone, Copy)]
struct PlaneBounds {
    min: Point<f64, Logical>,
    max: Point<f64, Logical>,
}

#[derive(Debug)]
struct PlaneAnimation {
    x: Animation,
    y: Animation,
}

impl Plane {
    pub(super) fn new(position: Point<f64, Logical>) -> Self {
        Self {
            position,
            scale: 1.,
            bounds: PlaneBounds {
                min: position,
                max: position,
            },
            animation: None,
        }
    }

    pub(super) fn position(&self) -> Point<f64, Logical> {
        self.animation.as_ref().map_or(self.position, |animation| {
            Point::from((animation.x.value(), animation.y.value()))
        })
    }

    pub(super) fn scale(&self) -> f64 {
        self.scale
    }

    pub(super) fn transform(&self) -> PlaneTransform {
        PlaneTransform {
            position: self.position(),
            scale: self.scale,
        }
    }

    pub(super) fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    pub(super) fn update_bounds(
        &mut self,
        viewport: Size<f64, Logical>,
        content: Size<f64, Logical>,
    ) {
        self.bounds = PlaneBounds {
            min: Point::from((-viewport.w, -viewport.h)),
            max: Point::from((content.w, content.h)),
        };
        self.position = self.clamp(self.position);
    }

    pub(super) fn set_position(&mut self, position: Point<f64, Logical>) {
        self.animation = None;
        self.position = self.clamp(position);
    }

    pub(super) fn offset(&mut self, delta: Point<f64, Logical>) {
        self.set_position(self.position() + delta);
    }

    pub(super) fn animate_to(
        &mut self,
        target: Point<f64, Logical>,
        clock: Clock,
        config: AnimationConfig,
    ) {
        let current = self.position();
        let target = self.clamp(target);
        self.position = target;
        self.animation = Some(PlaneAnimation {
            x: Animation::new(clock.clone(), current.x, target.x, 0., config),
            y: Animation::new(clock, current.y, target.y, 0., config),
        });
    }

    pub(super) fn advance_animations(&mut self) {
        if self
            .animation
            .as_ref()
            .is_some_and(|animation| animation.x.is_done() && animation.y.is_done())
        {
            self.animation = None;
        }
    }

    pub(super) fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub(super) fn output_delta_to_world(
        &self,
        delta: Point<f64, Logical>,
        viewport: Size<f64, Logical>,
    ) -> Point<f64, Logical> {
        self.transform().output_to_world(delta, viewport)
            - self.transform().output_to_world(Point::default(), viewport)
    }

    fn clamp(&self, position: Point<f64, Logical>) -> Point<f64, Logical> {
        Point::from((
            position.x.clamp(self.bounds.min.x, self.bounds.max.x),
            position.y.clamp(self.bounds.min.y, self.bounds.max.y),
        ))
    }
}

impl PlaneTransform {
    fn world_to_output(
        self,
        point: Point<f64, Logical>,
        viewport: Size<f64, Logical>,
    ) -> Point<f64, Logical> {
        let scaled_viewport = viewport.upscale(self.scale);
        let center_offset = (viewport.to_point() - scaled_viewport.to_point()).downscale(2.);
        (point - self.position).upscale(self.scale) + center_offset
    }

    fn output_to_world(
        self,
        point: Point<f64, Logical>,
        viewport: Size<f64, Logical>,
    ) -> Point<f64, Logical> {
        let scaled_viewport = viewport.upscale(self.scale);
        let center_offset = (viewport.to_point() - scaled_viewport.to_point()).downscale(2.);
        (point - center_offset).downscale(self.scale) + self.position
    }

    pub(super) fn row_geometry(
        self,
        row: usize,
        stride: f64,
        viewport: Size<f64, Logical>,
    ) -> Rectangle<f64, Logical> {
        Rectangle::new(
            self.world_to_output(Point::from((0., row as f64 * stride)), viewport),
            viewport.upscale(self.scale),
        )
    }
}
