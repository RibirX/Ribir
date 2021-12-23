use crate::prelude::*;
use crate::widget::theme::CheckboxTheme;

/// Represents a control that a user can select and clear.
#[stateful(custom)]
#[derive(Default, Clone)]
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
      size,
      border_width,
      check_mark_width,
      border_radius,
      border_color,
      checked_path,
      marker_color,
      color,
      indeterminate_path,
    } = self.style.clone();

    let has_check = self.indeterminate || self.checked;
    let radius = BorderRadius::all(Vector::new(border_radius, border_radius));
    let margin = EdgeInsets::all(4.);

    if has_check {
      declare! {
        CheckboxMarker {
          size: size + border_width * 2.,
          color: marker_color,
          path_width: if self.indeterminate {
            border_width
          } else {
            check_mark_width
          },
          path: if self.indeterminate {
            indeterminate_path
          } else {
            checked_path
          },
          margin,
          radius,
          background: color,
        }
      }
    } else {
      declare! {
        SizedBox {
          size: Size::new(size, size),
          margin,
          radius,
          border if !has_check =>: Border::all(BorderSide {
            color: border_color,
            width: border_width,
          }),
        }
      }
    }
  }
}

impl CombinationWidget for StatefulCheckbox {
  fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
    let w = self.0.borrow().clone();
    w.with_cursor(CursorIcon::Hand.into())
      .on_tap({
        let state = self.ref_cell();
        move |_| state.borrow_mut().switch_check()
      })
      .on_key_up({
        let state = self.ref_cell();
        move |k| {
          if k.key == VirtualKeyCode::Space {
            state.borrow_mut().switch_check()
          }
        }
      })
      .box_it()
  }
}

impl IntoStateful for StatefulCheckbox {
  type S = Self;

  #[inline]
  fn into_stateful(self) -> Self::S { self }
}

/// Build checkbox as stateful default to support user interactive.
impl Declare for Checkbox {
  type Builder = Checkbox;
}

impl DeclareBuilder for Checkbox {
  type Target = StatefulCheckbox;

  #[inline]
  fn build(self) -> Self::Target { self.into_stateful() }
}
// todo: use a common path widget to replace this.
#[stateful]
#[derive(Debug, Clone, Declare)]
pub struct CheckboxMarker {
  path_width: f32,
  path: Path,
  color: Color,
  size: f32,
}

impl RenderWidget for CheckboxMarker {
  type RO = Self;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.clone() }

  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    if self.size != object.size {
      ctx.mark_needs_layout();
    }
    object.size = self.size;
  }
}

impl RenderObject for CheckboxMarker {
  fn perform_layout(&mut self, clamp: BoxClamp, _: &mut RenderCtx) -> Size {
    Size::new(self.size, self.size).clamp(clamp.min, clamp.max)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    ctx
      .painter()
      .set_style(self.color.clone())
      .set_line_width(self.path_width)
      .stroke_path(self.path.clone());
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn layout() {
    let w = Checkbox::default().build();
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
    let c = Checkbox {
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
