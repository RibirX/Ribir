use ribir_core::prelude::*;

use crate::prelude::*;

/// Divider is a thin horizontal or vertical line, with padding on either side.
///
/// # example
/// ```
/// # use ribir_core::prelude::*;
/// # use ribir_widgets::prelude::{Divider, SizedBox, Direction, Column};
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
#[derive(Default, Query, Declare)]
pub struct Divider {
  #[declare(default = 1.)]
  // Extent of divider
  pub extent: f32,
  // Color of divider
  #[declare(default=Palette::of(ctx!()).outline_variant())]
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
      Size::new(clamp.max.width, self.extent)
    } else {
      Size::new(self.extent, clamp.max.height)
    }
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let mut size = ctx.box_size().unwrap();
    let (origin, size) = if self.direction.is_horizontal() {
      size.width -= self.indent + self.end_indent;
      size.height = self.thickness;
      let y = (self.extent - self.thickness) / 2.;
      (Point::new(self.indent, y), size)
    } else {
      size.width = self.thickness;
      size.height -= self.indent + self.end_indent;
      let x = (self.extent - self.thickness) / 2.;
      (Point::new(x, self.indent), size)
    };
    let painter = ctx.painter();
    painter.set_brush(self.color.clone());
    painter.rect(&Rect::new(origin, size));
    painter.fill();
  }
}
