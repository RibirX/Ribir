use std::cell::RefCell;

use ribir_core::prelude::*;

use crate::overlay::{AutoClosePolicy, Overlay, OverlayStyle};

pub fn default_tooltip_provider() -> Provider {
  Provider::new(CustomTooltip(Box::new(compose_custom_tooltip)))
}

struct OverlayTooltip {
  overlay: Overlay,
  wnd: Rc<Window>,
  host: TrackId,
  hovered: Box<dyn StateWatcher<Value = bool>>,
}

impl OverlayTooltip {
  fn new(tooltip: TextValue, wnd: Rc<Window>, host: TrackId) -> Self {
    let mut root = FatObj::new(
      follow! {
        target: $clone(host),
        x_align: AnchorX::center(),
        y_align: AnchorY::above(),
        @Text {
          text: tooltip,
          class: TOOLTIP,
        }
      }
      .into_widget(),
    );
    let hovered = root.is_hovered().clone_boxed_watcher();
    let tooltip = Reusable::new(root.into_widget());
    let overlay = Overlay::new(
      move || tooltip.get_widget(),
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    );
    Self { overlay, wnd, host, hovered }
  }

  fn hovered_watcher(&self) -> Box<dyn StateWatcher<Value = bool>> {
    self.hovered.clone_boxed_watcher()
  }
}

impl TooltipControl for OverlayTooltip {
  fn show(&mut self) {
    if self.host.get().is_some() {
      tracing::trace!(target = ?self.host.get(), "showing overlay tooltip");
      self.overlay.show(self.wnd.clone());
    } else {
      tracing::trace!("tooltip show requested after host was disposed; closing overlay");
      self.overlay.close();
    }
  }

  fn hide(&mut self) {
    tracing::trace!("hiding overlay tooltip");
    self.overlay.close();
  }

  fn is_showing(&self) -> bool { self.overlay.is_showing() }
}

fn compose_custom_tooltip<'c>(
  child: Widget<'c>, text: TextValue,
) -> (Widget<'c>, Box<dyn TooltipControl>) {
  let mut child = FatObj::new(child);
  let wnd = BuildCtx::get().window();
  let control = Rc::new(RefCell::new(OverlayTooltip::new(text, wnd, child.track_id())));
  let tooltip_hovered = control.borrow().hovered_watcher();
  let child = Tooltip::bind_hover_focus_with_tooltip(child, tooltip_hovered, control.clone());
  (child, Box::new(control))
}

#[cfg(test)]
mod tests {
  use ribir_core::{reset_test_env, test_helper::*};

  use super::*;

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

    let before = wnd.children_count(wnd.root());
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    let after = wnd.children_count(wnd.root());

    assert!(after > before, "custom tooltip should mount overlay content when hovered");

    wnd.process_cursor_move(Point::new(100., 70.));
    wnd.draw_frame();
    assert_eq!(wnd.children_count(wnd.root()), before);
  }

  #[test]
  fn custom_tooltip_keeps_overlay_mounted_while_text_changes() {
    reset_test_env!();

    let (tooltip_text, tooltip_text_writer) = split_value(String::new());
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            tooltip: pipe!($read(tooltip_text).clone()),
          }
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.children_count(wnd.root());
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    let shown = wnd.children_count(wnd.root());

    assert!(shown > before, "custom tooltip should show even when text starts empty");

    *tooltip_text_writer.write() = "tip".into();
    wnd.draw_frame();
    assert_eq!(wnd.children_count(wnd.root()), shown);

    wnd.process_cursor_move(Point::new(100., 70.));
    wnd.draw_frame();
    assert_eq!(wnd.children_count(wnd.root()), before);

    *tooltip_text_writer.write() = "tip 2".into();
    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    assert_eq!(wnd.children_count(wnd.root()), shown);
  }

  #[test]
  fn custom_tooltip_provider_supports_manual_control() {
    reset_test_env!();

    let tooltip = Tooltip::new("tip");
    let tooltip_in_widget = tooltip.clone();
    let wnd = TestWindow::new_with_size(
      fn_widget! {
        let tooltip = tooltip_in_widget.clone();
        @Providers {
          providers: [default_tooltip_provider()],
          @MockBox {
            size: Size::new(40., 20.),
            tooltip: tooltip,
          }
        }
      },
      Size::new(120., 80.),
    );
    wnd.draw_frame();

    let before = wnd.children_count(wnd.root());
    tooltip.show();
    wnd.draw_frame();
    let shown = wnd.children_count(wnd.root());

    assert!(shown > before, "manual show should mount overlay content");
    assert!(tooltip.is_showing());

    tooltip.hide();
    wnd.draw_frame();
    assert_eq!(wnd.children_count(wnd.root()), before);
    assert!(!tooltip.is_showing());
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

    wnd.process_cursor_move(Point::new(110., 90.));
    wnd.draw_frame();

    let overlay_root = wnd
      .children(wnd.root())
      .last()
      .expect("overlay tooltip should be mounted at window root");
    let overlay_bubble = wnd
      .children(overlay_root)
      .last()
      .unwrap_or(overlay_root);
    let overlay_size = wnd
      .widget_size(overlay_bubble)
      .expect("overlay tooltip should have a layout size");
    let overlay_pos = wnd
      .widget_pos(overlay_bubble)
      .expect("overlay tooltip should have a layout position");

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

    let before = wnd.children_count(wnd.root());
    wnd.process_cursor_move(Point::new(110., 90.));
    wnd.draw_frame();
    let shown = wnd.children_count(wnd.root());
    assert!(shown > before, "tooltip should mount while host is hovered");

    let overlay_root = wnd
      .children(wnd.root())
      .last()
      .expect("overlay tooltip should be mounted at window root");
    let overlay_bubble = wnd
      .children(overlay_root)
      .last()
      .unwrap_or(overlay_root);
    let overlay_global_pos = wnd.map_to_global(Point::zero(), overlay_bubble);
    let overlay_size = wnd
      .widget_size(overlay_bubble)
      .expect("overlay tooltip should have a layout size");

    wnd.process_cursor_move(Point::new(
      overlay_global_pos.x + overlay_size.width / 2.,
      overlay_global_pos.y + overlay_size.height / 2.,
    ));
    wnd.draw_frame();

    assert_eq!(
      wnd.children_count(wnd.root()),
      shown,
      "moving onto tooltip content should keep tooltip visible"
    );

    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    assert_eq!(wnd.children_count(wnd.root()), before);
  }

  #[test]
  fn custom_tooltip_repositions_correctly_on_second_show() {
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

    let bubble_rect = || {
      let overlay_root = wnd
        .children(wnd.root())
        .last()
        .expect("overlay tooltip should be mounted at window root");
      let overlay_bubble = wnd
        .children(overlay_root)
        .last()
        .unwrap_or(overlay_root);
      let pos = wnd
        .widget_pos(overlay_bubble)
        .expect("overlay bubble should have position");
      let size = wnd
        .widget_size(overlay_bubble)
        .expect("overlay bubble should have size");
      (pos, size)
    };

    wnd.process_cursor_move(Point::new(110., 90.));
    wnd.draw_frame();
    let (first_pos, first_size) = bubble_rect();

    wnd.process_cursor_move(Point::new(10., 10.));
    wnd.draw_frame();
    wnd.process_cursor_move(Point::new(110., 90.));
    wnd.draw_frame();
    let (second_pos, second_size) = bubble_rect();

    assert_eq!(first_pos, second_pos);
    assert_eq!(first_size, second_size);
  }
}
