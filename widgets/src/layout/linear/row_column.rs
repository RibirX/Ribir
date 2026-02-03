use ribir_core::prelude::*;

use super::{Align, Direction, JustifyContent};

/// A horizontal layout container that arranges children sequentially in a row.
///
/// Provides basic flexbox-like layout capabilities for horizontal arrangements
/// with simpler configuration than the full [`Flex`] container.
///
/// # Features
/// - Controls vertical alignment of items with `align_items`
/// - Distributes horizontal space between items with `justify_content`
///
/// # Limitations
/// - No support for wrapping items to new lines
/// - No flexible item sizing (all items use their intrinsic width)
/// - Single-axis alignment only
///
/// For complex layouts, use [`Flex`] directly instead.
#[derive(Declare, Default, MultiChild)]
pub struct Row {
  /// Vertical alignment of children within the container's height.
  ///
  /// When set to [`Align::Stretch`], children will expand to match the
  /// container's height (if constrained).
  #[declare(default)]
  pub align_items: Align,

  /// Distribution of remaining horizontal space between children.
  ///
  /// Controls how extra space is allocated when the total children width
  /// is less than the container's available width.
  #[declare(default)]
  pub justify_content: JustifyContent,
}

/// A vertical layout container that arranges children sequentially in a column.
///
/// Vertical counterpart to [`Row`], providing similar layout capabilities
/// for vertical arrangements. See [`Row`] documentation for detailed
/// behavior descriptions.
#[derive(Declare, Default, MultiChild)]
pub struct Column {
  /// Horizontal alignment of children within the container's width.
  ///
  /// When set to [`Align::Stretch`], children will expand to match the
  /// container's width (if constrained).
  #[declare(default)]
  pub align_items: Align,

  /// Distribution of remaining vertical space between children.
  ///
  /// Controls how extra space is allocated when the total children height
  /// is less than the container's available height.
  #[declare(default)]
  pub justify_content: JustifyContent,
}

impl Render for Row {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    perform_linear_measure(
      Direction::Horizontal,
      self.align_items,
      self.justify_content,
      clamp,
      ctx,
    )
  }

  fn place_children(&self, _size: Size, ctx: &mut PlaceCtx) {
    perform_linear_layout_positions(
      Direction::Horizontal,
      self.align_items,
      self.justify_content,
      _size,
      ctx,
    )
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("row") }
}

impl Render for Column {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    perform_linear_measure(Direction::Vertical, self.align_items, self.justify_content, clamp, ctx)
  }

  fn place_children(&self, _size: Size, ctx: &mut PlaceCtx) {
    perform_linear_layout_positions(
      Direction::Vertical,
      self.align_items,
      self.justify_content,
      _size,
      ctx,
    )
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> { std::borrow::Cow::Borrowed("column") }
}

/// Core measure algorithm for linear arrangements (both rows and columns).
///
/// Implements the measurement phase:
/// 1. Calculate total content size and child constraints
///
/// # Parameters
/// - `dir`: Layout direction (horizontal/vertical)
/// - `align_items`: Cross-axis alignment strategy
/// - `justify_content`: Main-axis space distribution
/// - `clamp`: Size constraints from parent
/// - `ctx`: Layout context for children measurement
fn perform_linear_measure(
  dir: Direction, align_items: Align, justify_content: JustifyContent, clamp: BoxClamp,
  ctx: &mut MeasureCtx,
) -> Size {
  let cross_max = dir.cross_max_of(&clamp);
  let child_clamp = if align_items == Align::Stretch && cross_max.is_finite() {
    dir.with_fixed_cross(clamp.loose(), cross_max)
  } else {
    dir.with_cross_max(clamp.loose(), cross_max)
  };

  let (ctx, children) = ctx.split_children();
  let (mut main, mut cross) = (0., 0f32);
  for child in children {
    let child_size = ctx.layout_child(child, child_clamp);
    main += dir.main_of(child_size);
    cross = cross.max(dir.cross_of(child_size));
  }

  let main_container = dir.container_main(&clamp, main);
  let main = dir.main_clamp(main, &clamp);
  let main = if justify_content.is_space_layout() { main_container } else { main };
  let cross = dir.cross_clamp(cross, &clamp);
  dir.to_size(main, cross)
}

/// Core layout algorithm for positioning children in linear arrangements.
///
/// # Parameters
/// - `dir`: Layout direction (horizontal/vertical)
/// - `align_items`: Cross-axis alignment strategy
/// - `justify_content`: Main-axis space distribution
/// - `size`: Container size (calculated in measure phase)
/// - `ctx`: Layout context for positioning children
fn perform_linear_layout_positions(
  dir: Direction, align_items: Align, justify_content: JustifyContent, size: Size,
  ctx: &mut PlaceCtx,
) {
  let child_cnt = ctx.children().count();
  let main_container = dir.main_of(size);
  let cross_container = dir.cross_of(size);

  // Calculate total main size from already-measured children
  let (ctx, children) = ctx.split_children();
  let total_main: f32 = children
    .map(|c| {
      let size = ctx.widget_box_size(c).unwrap();
      dir.main_of(size)
    })
    .sum();
  let (mut main_pos, step) =
    justify_content.item_offset_and_step(main_container - total_main, child_cnt);

  let (ctx, children) = ctx.split_children();
  for child in children {
    let child_size = ctx.widget_box_size(child).unwrap();
    let cross_pos = align_items.align_value(dir.cross_of(child_size), cross_container);

    let pos = dir.to_point(main_pos, cross_pos);
    ctx.update_position(child, pos);
    main_pos += dir.main_of(child_size) + step;
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_layout_test!(
    row_stretch_height,
    WidgetTester::new(fn_widget! {
      @Row {
        align_items: Align::Stretch,
        @Container { size: Size::new(100., 50.) }
      }
    })
    .with_wnd_size(Size::new(500., 200.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(100., 200.))
  );
}
