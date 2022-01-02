use crate::prelude::*;
use crate::widget::theme::CheckboxTheme;

/// Represents a control that a user can select and clear.
#[stateful(custom)]
#[derive(Default, Clone, Declare)]
pub struct Checkbox {
  pub checked: bool,
  pub indeterminate: bool,
  pub style: CheckboxTheme,
}

impl Checkbox {
  fn switch_check(&mut self) {
    if self.indeterminate {
      self.indeterminate = false;
      self.checked = false;
    } else {
      self.checked = !self.checked;
    }
  }
}

impl CombinationWidget for Checkbox {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    let CheckboxTheme {
      mut size,
      border_width,
      radius,
      border_color,
      checked_path,
      check_background: color,
      indeterminate_path,
    } = self.style.clone();

    let has_checked = self.indeterminate || self.checked;
    // border draw out of the box
    if !has_checked {
      size += border_width * 2.;
    }

    declare! {
      SizedBox {
        size: Size::new(size, size),
        margin: EdgeInsets::all(4.),
        radius: Radius::all(radius),
        border if !has_checked =>: Border::all(BorderSide {
          color: border_color,
          width: border_width,
        }),
        background if has_checked => : color,
        has_checked.then(||{
          if self.indeterminate {
            indeterminate_path
          } else {
            checked_path
          }
        })
      }
    }
  }
}

impl CombinationWidget for StatefulCheckbox {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    self
      .0
      .clone()
      .with_cursor(CursorIcon::Hand.into())
      .on_tap({
        let mut state = self.state_ref();
        move |_| state.switch_check()
      })
      .on_key_up({
        let mut state = self.state_ref();
        move |k| {
          if k.key == VirtualKeyCode::Space {
            state.switch_check()
          }
        }
      })
      .box_it()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn layout() {
    let w = CheckboxBuilder::default().build();
    let (rect, child) = widget_and_its_children_box_rect(w.box_it(), Size::new(200., 200.));
    debug_assert_eq!(rect, Rect::new(Point::new(0., 0.), Size::new(24., 24.)));

    debug_assert_eq!(
      child,
      vec![Rect::new(Point::new(4., 4.), Size::new(16., 16.))]
    );
  }

  #[test]
  #[ignore = "gpu need"]
  fn checked_paint() {
    let c = Checkbox { checked: true, ..<_>::default() };
    let mut window = window::Window::headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(window.render(), "../test/test_imgs/checkbox_checked.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn unchecked_paint() {
    let mut window =
      window::Window::headless(Checkbox::default().box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(window.render(), "../test/test_imgs/checkbox_uncheck.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn indeterminate_paint() {
    let c = CheckboxBuilder {
      checked: true,
      indeterminate: true,
      ..<_>::default()
    }
    .build();
    let mut window = window::Window::headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.render(),
      "../test/test_imgs/checkbox_indeterminate.png"
    );

    let c = Checkbox {
      checked: false,
      indeterminate: true,
      ..<_>::default()
    };
    let mut window = window::Window::headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.render(),
      "../test/test_imgs/checkbox_indeterminate.png"
    );
  }
}
