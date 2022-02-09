use crate::prelude::*;
use crate::widget::theme::CheckboxTheme;

/// Represents a control that a user can select and clear.
#[derive(Default, Clone, Declare)]
pub struct Checkbox {
  pub checked: bool,
  pub indeterminate: bool,
  pub style: CheckboxTheme,
}

impl Checkbox {
  pub fn switch_check(&mut self) {
    if self.indeterminate {
      self.indeterminate = false;
      self.checked = false;
    } else {
      self.checked = !self.checked;
    }
  }
}

impl StatefulCombination for Checkbox {
  fn build(this: &Stateful<Self>, _: &mut BuildCtx) -> BoxedWidget {
    let CheckboxTheme {
      mut size,
      border_width,
      radius,
      border_color,
      checked_path,
      check_background: color,
      indeterminate_path,
    } = this.style.clone();

    let has_checked = this.indeterminate || this.checked;
    // border draw out of the box
    if has_checked {
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
        background if has_checked =>: color,
        cursor: CursorIcon::Hand,
        on_tap: {
          let mut state = unsafe { this.state_ref() };
          move |_| state.switch_check()
        },
        on_key_up: {
          let mut state = unsafe { this.state_ref() };
          move |k| {
            if k.key == VirtualKeyCode::Space {
              state.switch_check()
            }
          }
        },

        has_checked.then(||{
          if this.indeterminate {
            indeterminate_path
          } else {
            checked_path
          }
        })
      }
    }
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
    let mut window = Window::wgpu_headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.painter_backend(),
      "../test/test_imgs/checkbox_checked.png"
    );
  }

  #[test]
  #[ignore = "gpu need"]
  fn unchecked_paint() {
    let mut window = Window::wgpu_headless(Checkbox::default().box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.painter_backend(),
      "../test/test_imgs/checkbox_uncheck.png"
    );
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
    let mut window = Window::wgpu_headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.painter_backend(),
      "../test/test_imgs/checkbox_indeterminate.png"
    );

    let c = Checkbox {
      checked: false,
      indeterminate: true,
      ..<_>::default()
    };
    let mut window = Window::wgpu_headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.painter_backend(),
      "../test/test_imgs/checkbox_indeterminate.png"
    );
  }
}
