use std::cell::RefCell;

use ribir_core::prelude::*;

pub fn default_tooltip_provider() -> Provider {
  Provider::new(CustomTooltip(Box::new(compose_custom_tooltip)))
}

struct OverlayTooltip {
  overlay: Overlay,
  wnd: Rc<Window>,
  host: TrackId,
}

impl OverlayTooltip {
  fn new(tooltip: TextValue, wnd: Rc<Window>, host: TrackId) -> Self {
    let tooltip = Reusable::new(follow! {
      target: $clone(host),
      x_align: AnchorX::center(),
      y_align: AnchorY::under(),
      @Text {
        text: tooltip,
        class: TOOLTIP,
      }
    });
    let overlay = Overlay::new(
      move || tooltip.get_widget(),
      OverlayStyle { auto_close_policy: AutoClosePolicy::NOT_AUTO_CLOSE, mask: None },
    );
    Self { overlay, wnd, host }
  }
}

impl TooltipControl for OverlayTooltip {
  fn show(&mut self) {
    if self.host.get().is_some() {
      self.overlay.show(self.wnd.clone());
    } else {
      self.overlay.close();
    }
  }

  fn hide(&mut self) { self.overlay.close(); }

  fn is_showing(&self) -> bool { self.overlay.is_showing() }
}

fn compose_custom_tooltip<'c>(
  child: Widget<'c>, text: TextValue,
) -> (Widget<'c>, Box<dyn TooltipControl>) {
  let mut child = FatObj::new(child);
  let wnd = BuildCtx::get().window();
  let control = Rc::new(RefCell::new(OverlayTooltip::new(text, wnd, child.track_id())));
  let child = Tooltip::bind_hover_focus(child, control.clone());
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
}
