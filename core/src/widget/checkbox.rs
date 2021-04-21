use crate::prelude::*;
use crate::widget::theme_data::CheckboxTheme;
use rxrust::prelude::*;

/// Represents a control that a user can select and clear.
#[derive(Debug, Widget, Default)]
pub struct Checkbox {
  pub checked: bool,
  pub indeterminate: bool,
  pub theme: CheckboxTheme,
}

impl Checkbox {
  pub fn from_theme(theme: &ThemeData) -> Self {
    Self {
      theme: theme.check_box.clone(),

      ..Default::default()
    }
  }

  #[inline]
  pub fn with_checked(mut self, checked: bool) -> Self {
    self.checked = checked;
    self
  }

  #[inline]
  pub fn with_indeterminate(mut self, b: bool) -> Self {
    self.indeterminate = b;
    self
  }

  fn switch_check(&mut self) {
    if self.indeterminate {
      self.indeterminate = false;
      self.checked = false;
    } else {
      self.checked = !self.checked;
    }
  }
}

impl Stateful<Checkbox> {
  /// A change stream of the checked state.
  pub fn checked_state(
    &mut self,
  ) -> impl LocalObservable<'static, Item = StateChange<bool>, Err = ()> {
    self.state_change(|w| w.checked)
  }
}

impl CombinationWidget for Checkbox {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    let CheckboxTheme {
      size,
      border_width,
      check_mark_width,
      border_radius,
      border_color,
      checked_path,
      marker_color,
      color,
    } = self.theme.clone();
    let marker = if self.indeterminate || self.checked {
      let size = size + border_width * 2.;
      let (path, check_mark_width) = if self.indeterminate {
        let center_y = size / 2.;
        let mut builder = PathBuilder::new();
        builder
          .begin_path(Point::new(3., center_y))
          .line_to(Point::new(size - 3., center_y))
          .close_path();
        (builder.build(), 2.)
      } else {
        (checked_path, check_mark_width)
      };
      CheckboxMarker {
        size,
        check_mark_width,
        color: marker_color,
        path,
      }
      .with_background(color.into())
    } else {
      SizedBox::empty_box(Size::new(size, size)).with_border(Border::all(BorderSide {
        color: border_color,
        width: border_width,
      }))
    }
    .with_border_radius(BorderRadius::all(Vector::new(border_radius, border_radius)))
    .with_margin(EdgeInsets::all(4.));

    let mut state = self.state_ref_cell(ctx);
    let mut state2 = state.clone();
    marker
      .on_tap(move |_| state.borrow_mut().switch_check())
      .on_key_up(move |k| {
        if k.key == VirtualKeyCode::Space {
          state2.borrow_mut().switch_check()
        }
      })
      .with_cursor(CursorIcon::Hand)
      .box_it()
  }
}

#[derive(Debug, Widget, Clone)]
pub struct CheckboxMarker {
  check_mark_width: f32,
  path: Path,
  color: Color,
  size: f32,
}

impl RenderWidget for CheckboxMarker {
  type RO = CheckboxMarker;

  #[inline]
  fn create_render_object(&self) -> Self::RO { self.clone() }

  #[inline]
  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> { None }
}

impl RenderObject for CheckboxMarker {
  type Owner = CheckboxMarker;

  fn update(&mut self, owner_widget: &Self::Owner, ctx: &mut UpdateCtx) {
    if (owner_widget.size - self.size).abs() < f32::EPSILON {
      ctx.mark_needs_layout();
    }
    *self = owner_widget.clone();
  }

  fn perform_layout(&mut self, clamp: BoxClamp, _: &mut RenderCtx) -> Size {
    Size::new(self.size, self.size).clamp(clamp.min, clamp.max)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    ctx
      .painter()
      .set_style(self.color.clone())
      .set_line_width(self.check_mark_width)
      .stroke_path(self.path.clone());
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;
  use widget::theme::material;

  fn checkbox() -> Checkbox { Checkbox::from_theme(&material::light("".to_string())) }
  #[test]
  fn layout() {
    let w = checkbox();
    let (rect, child) = widget_and_its_children_box_rect(w, Size::new(200., 200.));
    debug_assert_eq!(rect, Rect::new(Point::new(0., 0.), Size::new(24., 24.)));

    debug_assert_eq!(
      child,
      vec![Rect::new(Point::new(4., 4.), Size::new(16., 16.))]
    );
  }

  #[test]
  #[ignore = "gpu need"]
  fn checked_paint() {
    let c = checkbox().with_checked(true);
    let mut window = window::Window::headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(window.render(), "../test/test_imgs/checkbox_checked.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn unchecked_paint() {
    let mut window = window::Window::headless(checkbox().box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(window.render(), "../test/test_imgs/checkbox_uncheck.png");
  }

  #[test]
  #[ignore = "gpu need"]
  fn indeterminate_paint() {
    let c = checkbox().with_checked(true).with_indeterminate(true);
    let mut window = window::Window::headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.render(),
      "../test/test_imgs/checkbox_indeterminate.png"
    );

    let c = checkbox().with_checked(false).with_indeterminate(true);
    let mut window = window::Window::headless(c.box_it(), DeviceSize::new(100, 100));
    window.render_ready();
    window.draw_frame();

    unit_test::assert_canvas_eq!(
      window.render(),
      "../test/test_imgs/checkbox_indeterminate.png"
    );
  }
}
