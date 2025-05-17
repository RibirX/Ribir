use ribir_core::prelude::*;

use super::{Direction, JustifyContent};

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
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    perform_linear_layout(Direction::Horizontal, self.align_items, self.justify_content, clamp, ctx)
  }
}

impl Render for Column {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    perform_linear_layout(Direction::Vertical, self.align_items, self.justify_content, clamp, ctx)
  }
}

/// Core layout algorithm for linear arrangements (both rows and columns).
///
/// Implements a two-phase layout process:
/// 1. **Measurement Phase**: Calculate total content size and child constraints
/// 2. **Placement Phase**: Position children according to alignment rules
///
/// # Parameters
/// - `dir`: Layout direction (horizontal/vertical)
/// - `align_items`: Cross-axis alignment strategy
/// - `justify_content`: Main-axis space distribution
/// - `clamp`: Size constraints from parent
/// - `ctx`: Layout context for children measurement
fn perform_linear_layout(
  dir: Direction, align_items: Align, justify_content: JustifyContent, clamp: BoxClamp,
  ctx: &mut LayoutCtx,
) -> Size {
  let cross_max = dir.cross_max_of(&clamp);
  let child_clamp = if align_items == Align::Stretch && cross_max.is_finite() {
    dir.with_fixed_cross(BoxClamp::default(), cross_max)
  } else {
    dir.with_cross_max(BoxClamp::default(), cross_max)
  };

  let (ctx, children) = ctx.split_children();
  let (mut main, mut cross) = (0., 0f32);
  for child in children {
    let child_size = ctx.perform_child_layout(child, child_clamp);
    main += dir.main_of(child_size);
    cross = cross.max(dir.cross_of(child_size));
  }

  let child_cnt = ctx.children().count();
  let main_container = dir.container_main(&clamp, main);
  let (mut main_pos, step) = justify_content.item_offset_and_step(main_container - main, child_cnt);

  let (ctx, children) = ctx.split_children();
  let cross = dir.cross_clamp(cross, &clamp);
  for child in children {
    let child_size = ctx.widget_box_size(child).unwrap();
    let cross_pos = align_items.align_value(dir.cross_of(child_size), cross);

    ctx.update_position(child, dir.to_point(main_pos, cross_pos));
    main_pos += dir.main_of(child_size) + step;
  }

  let main = dir.main_clamp(main, &clamp);
  let main = if justify_content.is_space_layout() { main_container } else { main };

  dir.to_size(main, cross)
}
