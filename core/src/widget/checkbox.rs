use crate::prelude::*;
use crate::widget::theme::CheckboxTheme;

/// Represents a control that a user can select and clear.
#[stateful(custom = "Checkbox")]
#[derive(Default)]
pub struct CheckboxInner {
  #[state]
  pub checked: bool,
  #[state]
  pub indeterminate: bool,
  pub theme: CheckboxTheme,
}

impl Checkbox {
  pub fn from_theme(theme: &Theme) -> Self {
    CheckboxInner {
      theme: theme.check_box.clone(),

      ..Default::default()
    }
    .into_stateful()
  }

  #[inline]
  pub fn with_checked(self, checked: bool) -> Self {
    self.borrow_mut().checked = checked;
    self
  }

  #[inline]
  pub fn with_indeterminate(self, b: bool) -> Self {
    self.borrow_mut().indeterminate = b;
    self
  }
}

impl CheckboxInner {
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
    } = self.0.borrow().theme.clone();
    let check_state = self.0.borrow();

    let state = self.ref_cell();
    let state2 = state.clone();

    let mut marker = BoxDecoration {
      radius: Some(BorderRadius::all(Vector::new(border_radius, border_radius))),
      ..<_>::default()
    };

    let checkbox = if check_state.indeterminate || check_state.checked {
      let size = size + border_width * 2.;
      let (path, check_mark_width) = if check_state.indeterminate {
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
      marker.background = Some(color.into());
      marker.have(
        CheckboxMarker {
          size,
          check_mark_width,
          color: marker_color,
          path,
        }
        .box_it(),
      )
    } else {
      marker.border = Some(Border::all(BorderSide {
        color: border_color,
        width: border_width,
      }));
      marker.have(SizedBox::from_size(Size::new(size, size)).box_it())
    };

    Margin::new(EdgeInsets::all(4.))
      .on_tap(move |_| state.borrow_mut().switch_check())
      .on_key_up(move |k| {
        if k.key == VirtualKeyCode::Space {
          state2.borrow_mut().switch_check()
        }
      })
      .with_cursor(CursorIcon::Hand)
      .have(checkbox.box_it())
      .box_it()
  }
}

#[stateful]
#[derive(Debug, Clone)]
pub struct CheckboxMarker {
  #[state]
  check_mark_width: f32,
  #[state]
  path: Path,
  #[state]
  color: Color,
  #[state]
  size: f32,
}

pub struct CheckboxMarkerRender(CheckboxMarkerState);

impl RenderWidget for CheckboxMarker {
  type RO = CheckboxMarkerRender;

  #[inline]
  fn create_render_object(&self) -> Self::RO { CheckboxMarkerRender(self.clone_states()) }
}

impl RenderObject for CheckboxMarkerRender {
  type States = CheckboxMarkerState;

  #[inline]
  fn update(&mut self, states: Self::States, _: &mut UpdateCtx) { self.0 = states; }

  fn perform_layout(&mut self, clamp: BoxClamp, _: &mut RenderCtx) -> Size {
    Size::new(self.0.size, self.0.size).clamp(clamp.min, clamp.max)
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  fn paint<'a>(&'a self, ctx: &mut PaintingContext<'a>) {
    ctx
      .painter()
      .set_style(self.0.color.clone())
      .set_line_width(self.0.check_mark_width)
      .stroke_path(self.0.path.clone());
  }

  #[inline]
  fn get_states(&self) -> &Self::States { &self.0 }
}

impl StatePartialEq<Self> for Path {
  #[inline]
  fn eq(&self, _: &Self) -> bool { false }
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
