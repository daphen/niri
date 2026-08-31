use std::time::Duration;

use insta::assert_snapshot;
use niri_config::animations::{Curve, EasingParams, Kind};
use niri_config::workspace::WorkspaceName;
use niri_config::{Config, Workspace};
use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::Layer;
use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, KeyboardInteractivity,
};

use super::*;
use crate::tests::client::{LayerConfigureProps, LayerMargin};

#[test]
fn workspace_bar_gap_compacts_during_keyboard_and_touchpad_switches() {
    const LINEAR: Kind = Kind::Easing(EasingParams {
        duration_ms: 1000,
        curve: Curve::Linear,
    });

    fn set_time(niri: &mut crate::niri::Niri, time: Duration) {
        let now = niri.clock.now();
        niri.clock.set_unadjusted(now);
        let _ = niri.clock.now();
        niri.clock.set_unadjusted(Duration::ZERO);
        niri.clock.set_rate(1.);
        let _ = niri.clock.now();
        niri.clock.set_unadjusted(time);
        let _ = niri.clock.now();
        niri.clock.set_rate(0.);
    }

    fn content_gap(niri: &crate::niri::Niri) -> f64 {
        let monitor = niri.layout.monitors().next().unwrap();
        let geos: Vec<_> = monitor.workspaces_render_geo().collect();
        geos[1].loc.y - geos[0].loc.y - 1022.
    }

    fn visible_workspaces(niri: &crate::niri::Niri) -> usize {
        niri.layout
            .monitors()
            .next()
            .unwrap()
            .workspaces_with_render_geo()
            .count()
    }

    let mut config = Config::default();
    config.layout.gaps = 4.;
    config.animations.workspace_switch.0.kind = LINEAR;
    config.animations.horizontal_view_movement.0.kind = LINEAR;
    for name in ["one", "two"] {
        config.workspaces.push(Workspace {
            name: WorkspaceName(name.to_owned()),
            open_on_output: None,
            layout: None,
        });
    }

    let mut f = Fixture::with_config(config);
    f.add_output(1, (1920, 1080));
    let id = f.add_client();
    let layer = f.client(id).create_layer(None, Layer::Top, "");
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Bottom),
        size: Some((0, 50)),
        exclusive_zone: Some(50),
        ..Default::default()
    });
    layer.commit();
    f.roundtrip(id);

    let layer = f.client(id).layer(&surface);
    layer.attach_new_buffer();
    layer.set_size(1920, 50);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    set_time(f.niri(), Duration::ZERO);
    assert_eq!(content_gap(f.niri()), 58.);
    assert_eq!(visible_workspaces(f.niri()), 1);

    f.niri().layout.switch_workspace_down();
    assert_eq!(content_gap(f.niri()), 58.);
    set_time(f.niri(), Duration::from_millis(250));
    f.niri().advance_animations();
    let keyboard_quarter_gap = content_gap(f.niri());
    assert!(0. < keyboard_quarter_gap && keyboard_quarter_gap < 58.);
    set_time(f.niri(), Duration::from_millis(500));
    f.niri().advance_animations();
    assert_eq!(content_gap(f.niri()), 0.);
    set_time(f.niri(), Duration::from_millis(750));
    f.niri().advance_animations();
    assert_eq!(content_gap(f.niri()), keyboard_quarter_gap);
    f.niri_complete_animations();
    assert_eq!(content_gap(f.niri()), 58.);
    assert_eq!(visible_workspaces(f.niri()), 1);

    let output = f.niri().layout.outputs().next().unwrap().clone();
    let idle_geos: Vec<_> = f
        .niri()
        .layout
        .monitors()
        .next()
        .unwrap()
        .workspaces_render_geo()
        .collect();
    f.niri()
        .layout
        .workspace_switch_gesture_begin(&output, true);
    let begin_geos: Vec<_> = f
        .niri()
        .layout
        .monitors()
        .next()
        .unwrap()
        .workspaces_render_geo()
        .collect();
    assert_eq!(begin_geos, idle_geos);
    f.niri()
        .layout
        .workspace_switch_gesture_update(0., Duration::from_millis(10), true);
    assert_eq!(content_gap(f.niri()), 58.);
    f.niri().layout.workspace_switch_gesture_end(Some(true));
    let release_geos: Vec<_> = f
        .niri()
        .layout
        .monitors()
        .next()
        .unwrap()
        .workspaces_render_geo()
        .collect();
    assert_eq!(release_geos, idle_geos);
    f.niri_complete_animations();

    f.niri()
        .layout
        .workspace_switch_gesture_begin(&output, true);
    f.niri()
        .layout
        .workspace_switch_gesture_update(-12., Duration::from_millis(20), true);
    let tiny_gap = content_gap(f.niri());
    assert!(0. < tiny_gap && tiny_gap < 58.);
    f.niri().layout.workspace_switch_gesture_end(Some(true));
    assert_eq!(content_gap(f.niri()), tiny_gap);
    f.niri_complete_animations();

    f.niri()
        .layout
        .workspace_switch_gesture_begin(&output, true);
    f.niri()
        .layout
        .workspace_switch_gesture_update(-600., Duration::from_millis(30), true);
    assert_eq!(content_gap(f.niri()), 0.);
    f.niri().layout.workspace_switch_gesture_end(Some(true));
    assert_eq!(content_gap(f.niri()), 0.);
    set_time(f.niri(), Duration::from_millis(1000));
    f.niri().advance_animations();
    let gesture_release_gap = content_gap(f.niri());
    assert!(0. < gesture_release_gap && gesture_release_gap < 58.);
    f.niri_complete_animations();
    assert_eq!(content_gap(f.niri()), 58.);
    assert_eq!(visible_workspaces(f.niri()), 1);
}

#[test]
fn simple_top_anchor() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));
    let id = f.add_client();

    let layer = f.client(id).create_layer(None, Layer::Top, "");
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 50)),
        ..Default::default()
    });
    layer.commit();
    f.roundtrip(id);

    let layer = f.client(id).layer(&surface);
    layer.attach_new_buffer();
    layer.set_size(100, 100);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 50");
}

#[test]
fn margin_overflow() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));
    let id = f.add_client();

    let layer = f.client(id).create_layer(None, Layer::Top, "");
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom),
        margin: Some(LayerMargin {
            top: i32::MAX,
            right: i32::MAX,
            bottom: i32::MAX,
            left: i32::MAX,
        }),
        exclusive_zone: Some(i32::MAX),
        ..Default::default()
    });
    layer.commit();
    f.roundtrip(id);

    let layer = f.client(id).layer(&surface);
    layer.attach_new_buffer();
    layer.set_size(100, 100);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"size: 0 × 0");

    // Add a second one for good measure.
    let layer = f.client(id).create_layer(None, Layer::Top, "");
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top | Anchor::Bottom),
        margin: Some(LayerMargin {
            top: i32::MAX,
            right: i32::MAX,
            bottom: i32::MAX,
            left: i32::MAX,
        }),
        exclusive_zone: Some(i32::MAX),
        ..Default::default()
    });
    layer.commit();
    f.roundtrip(id);

    let layer = f.client(id).layer(&surface);
    layer.attach_new_buffer();
    layer.set_size(100, 100);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"size: 0 × 0");
}

#[test]
fn unmap_through_null_buffer() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));
    let id = f.add_client();

    let layer = f.client(id).create_layer(None, Layer::Top, "");
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 50)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 50");

    layer.attach_new_buffer();
    layer.set_size(100, 100);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // No new configure since nothing changed.
    assert_snapshot!(layer.format_recent_configures(), @"");

    // Unmap by attaching a null buffer. This moves the surface back to pre-initial-commit stage.
    layer.attach_null();
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // Configures must be empty because we haven't done an initial commit yet.
    assert_snapshot!(layer.format_recent_configures(), @"");

    // Do the initial commit again.
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 100)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // This is the new initial configure.
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 100");

    layer.attach_new_buffer();
    layer.set_size(100, 100);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"");
}

#[test]
fn multiple_commits_before_mapping() {
    let mut f = Fixture::new();
    f.add_output(1, (1920, 1080));
    let id = f.add_client();

    let layer = f.client(id).create_layer(None, Layer::Top, "");
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 50)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 50");

    // Change something that won't cause a configure.
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 50)),
        kb_interactivity: Some(KeyboardInteractivity::OnDemand),
        ..Default::default()
    });
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // No new configure since the size hasn't changed.
    assert_snapshot!(layer.format_recent_configures(), @"");

    // Change something that will cause a configure.
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 100)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // Configure with new size.
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 100");

    // Map.
    layer.attach_new_buffer();
    layer.set_size(100, 100);
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // No new configure since nothing changed.
    assert_snapshot!(layer.format_recent_configures(), @"");

    // Unmap by attaching a null buffer. This moves the surface back to pre-initial-commit stage.
    layer.attach_null();
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // Configures must be empty because we haven't done an initial commit yet.
    assert_snapshot!(layer.format_recent_configures(), @"");

    // Same configure props as before, but since we unmapped, we should get a new initial
    // configure (that will happen to match the previous configure we had got while mapped).
    let surface = layer.surface.clone();
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 100)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 100");

    // Change something that won't cause a configure.
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 100)),
        kb_interactivity: Some(KeyboardInteractivity::OnDemand),
        ..Default::default()
    });
    layer.ack_last_and_commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // No new configure since the size hasn't changed.
    assert_snapshot!(layer.format_recent_configures(), @"");

    // Change something that will cause a configure.
    layer.set_configure_props(LayerConfigureProps {
        anchor: Some(Anchor::Left | Anchor::Right | Anchor::Top),
        size: Some((0, 50)),
        ..Default::default()
    });
    layer.commit();
    f.double_roundtrip(id);

    let layer = f.client(id).layer(&surface);
    // Configure with new size.
    assert_snapshot!(layer.format_recent_configures(), @"size: 1920 × 50");
}
