use ribir_core::prelude::*;

use crate::prelude::*;

/// Divider is a thin horizontal or vertical line, with padding on either side.
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::*;
///
/// // use default Divider default settings
/// let widget = fn_widget! {
///   @Column {
///     @SizedBox { size: Size::new(10., 0.) }
///     @Divider { extent: 20. }
///     @SizedBox { size: Size::new(10., 0.) }
///   }
/// };
///
/// // use custom settings
/// let widget = fn_widget! {
///   @Column {
///     @SizedBox { size: Size::new(10., 0.) }
///     @Divider {
///       extent: 20.,
///       color: Color::RED,
///       direction: Direction::Horizontal,
///       // Thickness of line
///       thickness: 2.,
///       // front indentation distance
///       indent: 10.,
///       // behind indentation distance
///       end_indent: 10.,
///     }
///     @SizedBox { size: Size::new(10., 0.) }
///   }
/// };
/// ```
#[derive(Default, Declare)]
pub struct Divider {
  #[declare(default = 1.)]
  // Extent of divider
  pub extent: f32,
  // Color of divider
  #[declare(default=Palette::of(BuildCtx::get()).outline_variant())]
  pub color: Brush,
  // Direction of divider
  #[declare(default=Direction::Horizontal)]
  pub direction: Direction,
  // Thickness of line
  #[declare(default = 1.)]
  pub thickness: f32,
  // front indentation distance
  #[declare(default = 0.)]
  pub indent: f32,
  // behind indentation distance
  #[declare(default = 0.)]
  pub end_indent: f32,
}

impl Render for Divider {
  fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
    if self.direction.is_horizontal() {
      let width = clamp.max.width;
      if width.is_finite() { Size::new(width, self.extent) } else { clamp.min }
    } else {
      let height = clamp.max.height;
      if height.is_finite() { Size::new(self.extent, height) } else { clamp.min }
    }
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let rect = self.paint_rect(ctx.box_size().unwrap());
    let painter = ctx.painter();
    painter.set_fill_brush(self.color.clone());
    painter.rect(&rect);
    painter.fill();
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> {
    let rect = self.paint_rect(ctx.box_size()?);
    Some(rect)
  }
}

impl Divider {
  fn paint_rect(&self, mut box_size: Size) -> Rect {
    if self.direction.is_horizontal() {
      box_size.width -= self.indent + self.end_indent;
      box_size.height = self.thickness;
      let y = (self.extent - self.thickness) / 2.;
      Rect::new(Point::new(self.indent, y), box_size)
    } else {
      box_size.width = self.thickness;
      box_size.height -= self.indent + self.end_indent;
      let x = (self.extent - self.thickness) / 2.;
      Rect::new(Point::new(x, self.indent), box_size)
    }
  }
}
