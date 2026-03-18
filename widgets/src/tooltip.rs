use ribir_core::prelude::*;

use crate::overlay::{AutoClosePolicy, Overlay, OverlayStyle};

class_names! {
  #[doc = "Class name for the widgets tooltip presentation shell"]
  TOOLTIP_SHELL,
}

struct OverlayTooltip;

impl CustomTooltip for OverlayTooltip {
  fn spawn_bubble(
    &self, bubble: Widget<'static>, visible: Stateful<bool>, host_track: TrackId,
  ) -> Box<dyn FnOnce()> {
    let reusable = Reusable::new(bubble);
    let overlay = Overlay::new(
      move || {
        let reusable = reusable.clone();
        class! { class: TOOLTIP_SHELL, @ { reusable.get_widget() } }
      },
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    );

    let sub = watch!(*$read(visible))
      .distinct_until_changed()
      .subscribe({
        let overlay = overlay.clone();
        let wnd = BuildCtx::get().window();
        move |visible| {
          if visible && host_track.get().is_some() {
            overlay.show(wnd.clone());
          } else {
            overlay.close();
          }
        }
      });

    Box::new(move || {
      overlay.close();
      sub.unsubscribe();
    })
  }
}

pub fn default_tooltip_provider() -> Provider {
  Provider::new(Box::new(OverlayTooltip) as Box<dyn CustomTooltip>)
}

#[cfg(test)]
mod tests {
  use ribir_core::{reset_test_env, test_helper::*, window::WindowFlags};

  use super::*;
  use crate::prelude::{AnimatedPresence, Interruption, cases};

  const TOOLTIP_SHOW_DELAY: Duration = Duration::from_millis(500);
  const TOOLTIP_HIDE_DELAY: Duration = Duration::from_millis(150);
  const HOST_POINT: Point = Point::new(10., 10.);
  const OUTSIDE_POINT: Point = Point::new(100., 70.);
  const OFFSET_HOST_POINT: Point = Point::new(110., 90.);

  fn wait_for_tooltip_show_delay() {
    AppCtx::run_until(AppCtx::timer(TOOLTIP_SHOW_DELAY + Duration::from_millis(20)));
    AppCtx::run_until_stalled();
  }

  fn wait_for_tooltip_hide_delay() {
    AppCtx::run_until(AppCtx::timer(TOOLTIP_HIDE_DELAY + Duration::from_millis(20)));
    AppCtx::run_until_stalled();
  }

  fn wait_for(duration: Duration) {
    AppCtx::run_until(AppCtx::timer(duration));
    AppCtx::run_until_stalled();
  }

  fn root_children_count(wnd: &TestWindow) -> usize { wnd.children_count(wnd.root()) }

  fn move_cursor_and_draw(wnd: &TestWindow, point: Point) {
    wnd.process_cursor_move(point);
    wnd.draw_frame();
  }

  fn hover_and_show(wnd: &TestWindow, point: Point) -> usize {
    move_cursor_and_draw(wnd, point);
    wait_for_tooltip_show_delay();
    wnd.draw_frame();
    root_children_count(wnd)
  }

  fn overlay_root(wnd: &TestWindow) -> WidgetId {
    wnd
      .children(wnd.root())
      .last()
      .expect("overlay tooltip should be mounted at window root")
  }

  fn overlay_rect(wnd: &TestWindow) -> (Point, Size) {
    let overlay_root = overlay_root(wnd);
    let pos = wnd
      .widget_pos(overlay_root)
      .expect("overlay tooltip should have a layout position");
    let size = wnd
      .widget_size(overlay_root)
      .expect("overlay tooltip should have a layout size");
    (pos, size)
  }

  fn overlay_center_global(wnd: &TestWindow) -> Point {
    let overlay_root = overlay_root(wnd);
    let global = wnd.map_to_global(Point::zero(), overlay_root);
    let size = wnd
      .widget_size(overlay_root)
      .expect("overlay tooltip should have a layout size");
    Point::new(global.x + size.width / 2., global.y + size.height / 2.)
  }

  fn install_animated_tooltip_shell_theme() {
    let mut theme = Theme::default();
    theme.classes.insert(TOOLTIP_SHELL, |w| {
      fn_widget! {
        let mut w = FatObj::new(w);
        let opacity = w.opacity();

        @AnimatedPresence {
          cases: cases! {
            state: opacity,
            true => 1.0,
            false => 0.0,
          },
          enter: EasingTransition {
            easing: easing::CubicBezierEasing::new(0., 0., 0.2, 1.),
            duration: Duration::from_millis(150),
          },
          leave: EasingTransition {
            easing: easing::CubicBezierEasing::new(0.4, 0., 1., 1.),
            duration: Duration::from_millis(150),
          },
          interruption: Interruption::Fluid,
          @ { w }
        }
      }
      .into_widget()
    });
    AppCtx::set_app_theme(theme);
  }

  #[test]
  fn custom_tooltip_provider_mounts_on_hover() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            tooltip: "tip",
          }
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = root_children_count(&wnd);
    let shown = hover_and_show(&wnd, HOST_POINT);

    assert!(shown > before, "custom tooltip should mount overlay content when hovered");

    move_cursor_and_draw(&wnd, OUTSIDE_POINT);
    assert_eq!(root_children_count(&wnd), shown);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert_eq!(root_children_count(&wnd), before);
  }

  #[test]
  fn custom_tooltip_provider_supports_widget_content() {
    reset_test_env!();

    let tooltip = Tooltip::from_widget(fn_widget! {
      @MockBox { size: Size::new(60., 24.) }
    });
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            tooltip,
          }
        }
      },
      Size::new(160., 120.),
    );
    wnd.draw_frame();

    hover_and_show(&wnd, HOST_POINT);

    let overlay_root = overlay_root(&wnd);
    let overlay_bubble = wnd
      .children(overlay_root)
      .last()
      .unwrap_or(overlay_root);
    let overlay_size = wnd
      .widget_size(overlay_bubble)
      .expect("overlay tooltip should have a layout size");

    assert_eq!(overlay_size, Size::new(60., 24.));
  }

  #[test]
  fn custom_tooltip_provider_positions_overlay_relative_to_host() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            x: 100.,
            y: 80.,
            tooltip: "tip",
          }
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();

    hover_and_show(&wnd, OFFSET_HOST_POINT);

    let (overlay_pos, overlay_size) = overlay_rect(&wnd);

    let host_center_x = 100. + 20.;
    let bubble_center_x = overlay_pos.x + overlay_size.width / 2.;
    assert!((bubble_center_x - host_center_x).abs() < 1.0);
    assert!(overlay_pos.y < 80.);
    assert!((overlay_pos.y + overlay_size.height - 80.).abs() < 1.0);
  }

  #[test]
  fn custom_tooltip_stays_visible_while_hovering_overlay() {
    reset_test_env!();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            x: 100.,
            y: 80.,
            tooltip: "tip",
          }
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();

    let before = root_children_count(&wnd);
    let shown = hover_and_show(&wnd, OFFSET_HOST_POINT);
    assert!(shown > before, "tooltip should mount while host is hovered");

    wnd.process_cursor_move(overlay_center_global(&wnd));
    wnd.draw_frame();

    assert_eq!(
      root_children_count(&wnd),
      shown,
      "moving onto tooltip content should keep tooltip visible"
    );

    move_cursor_and_draw(&wnd, HOST_POINT);
    assert_eq!(root_children_count(&wnd), shown);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();
    assert_eq!(root_children_count(&wnd), before);
  }

  #[test]
  fn custom_tooltip_survives_second_hover_with_material_shell() {
    reset_test_env!();

    install_animated_tooltip_shell_theme();

    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            x: 100.,
            y: 80.,
            tooltip: "tip",
          }
        }
      },
      Size::new(240., 200.),
    );
    wnd.draw_frame();

    let before = root_children_count(&wnd);

    for cycle in 0..2 {
      let shown = hover_and_show(&wnd, OFFSET_HOST_POINT);
      assert!(shown > before, "tooltip should mount in hover cycle {cycle}");

      move_cursor_and_draw(&wnd, HOST_POINT);
      wait_for_tooltip_hide_delay();
      wnd.draw_frame();
      assert_eq!(root_children_count(&wnd), before);
    }
  }

  #[test]
  fn custom_tooltip_leave_animation_keeps_bubble_pinned() {
    reset_test_env!();

    install_animated_tooltip_shell_theme();

    let bubble_track = Stateful::new(None::<TrackId>);
    let bubble_track_reader = bubble_track.clone_reader();
    let wnd = TestWindow::new(
      fn_widget! {
        let mut bubble = @MockBox {
          size: Size::new(60., 24.),
        };
        let bubble_id = bubble.track_id();
        bubble.on_mounted(move |_| *$write(bubble_track) = Some($clone(bubble_id)));

        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            x: 100.,
            y: 80.,
            tooltip: Tooltip::from_widget(bubble),
          }
        }
      },
      Size::new(240., 200.),
      WindowFlags::ANIMATIONS,
    );
    wnd.draw_frame();

    let bubble_pos = || {
      let bubble = bubble_track_reader
        .read()
        .clone()
        .and_then(|track| track.get())
        .expect("tooltip bubble should stay mounted while visible or leaving");
      wnd.map_to_global(Point::zero(), bubble)
    };

    hover_and_show(&wnd, OFFSET_HOST_POINT);
    let shown_pos = bubble_pos();

    move_cursor_and_draw(&wnd, HOST_POINT);
    wait_for_tooltip_hide_delay();
    wnd.draw_frame();

    let leave_start = bubble_pos();
    assert_eq!(
      leave_start, shown_pos,
      "tooltip should start leave animation from its shown position"
    );

    wait_for(Duration::from_millis(60));
    wnd.draw_frame();
    let leave_mid = bubble_pos();
    assert_eq!(leave_mid, shown_pos, "tooltip should remain pinned during leave animation");
  }
}
