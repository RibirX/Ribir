use crate::prelude::*;
use crate::widget::theme_data::CheckboxTheme;

#[derive(Debug, Default)]
pub struct Checkbox {
  pub color: Color,
  pub marker_color: Color,
  pub checked: bool,
  pub indeterminate: bool,
  pub theme: CheckboxTheme,
}

impl Checkbox {
  pub fn from_theme(theme: ThemeData) -> Self {
    Self {
      color: theme.primary,
      theme: theme.check_box,
      marker_color: theme.secondary,
      ..Default::default()
    }
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

impl CombinationWidget for Checkbox {
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget {
    let CheckboxTheme {
      size,
      border_width,
      check_mark_width,
      border_radius,
      border_color,
      checked_path,
    } = self.theme.clone();
    let marker = if self.indeterminate || self.checked {
      let (path, check_mark_width) = if self.indeterminate {
        let center_y = size / 2.;
        let mut builder = PathBuilder::new();
        builder
          .begin_path(Point::new(3., center_y))
          .line_to(Point::new(size - 6., center_y));
        (builder.build(), 2.)
      } else {
        (checked_path, check_mark_width)
      };
      CheckboxMarker {
        size,
        check_mark_width,
        color: self.marker_color.clone(),
        path,
      }
      .with_background(self.color.clone().into())
    } else {
      SizedBox::empty_box(Size::new(size, size)).with_border(Border::all(BorderSide {
        color: border_color,
        width: border_width,
      }))
    }
    .with_border_radius(BorderRadius::all(Vector::new(border_radius, border_radius)))
    .with_margin(EdgeInsets::all(4.));

    let mut state = self.self_state_ref(ctx);
    marker
      .on_tap(move |_| state.borrow_mut().switch_check())
      .on_key_up(|_| unimplemented!())
      .box_it()
  }
}

impl_widget_for_combination_widget!(Checkbox);

#[derive(Debug, Clone)]
pub struct CheckboxMarker {
  check_mark_width: f32,
  path: Path,
  color: Color,
  size: f32,
}

impl_widget_for_render_widget!(CheckboxMarker);

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
    if owner_widget.size != self.size {
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
