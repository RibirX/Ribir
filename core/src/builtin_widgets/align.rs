use crate::{prelude::*, wrap_render::*};

/// A enum that describe how widget align to its box.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
  /// The children are aligned to the start edge of the box provided by parent.
  /// The same as [`HAlign::Left`]! if direction is horizontal and
  /// [`VAlign::Top`]! if direction is vertical.
  #[default]
  Start,
  /// The children are aligned to the center of the line of the box provide by
  /// parent. The same as [`HAlign::Center`]! if direction is horizontal and
  /// [`VAlign::Center`]! if direction is vertical.
  Center,
  /// The children are aligned to the start edge of the box provided by parent.
  /// The same as [`HAlign::Right`]! if direction is horizontal and
  /// [`VAlign::Bottom`]! if direction is vertical.
  End,
  /// Require the children to fill the whole box of one axis. This causes the
  /// constraints passed to the children to be tight. The same as
  /// [`HAlign::Stretch`]! if direction is horizontal and [`VAlign::Stretch`]!
  /// if direction is vertical.
  Stretch,
}

/// A enum that describe how widget align to its box in x-axis.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HAlign {
  /// The children are aligned to the left edge of the box provided by parent.
  #[default]
  Left,
  /// The children are aligned to the x-center of the box provide by parent.
  Center,
  /// The children are aligned to the right edge of the box provided by parent.
  Right,
  /// Require the children to fill the whole box in x-axis. This causes the
  /// constraints passed to the children to be tight.
  Stretch,
}

/// A enum that describe how widget align to its box in y-axis.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VAlign {
  #[default]
  /// The children are aligned to the top edge of the box provided by parent.
  Top,
  /// The children are aligned to the y-center of the box provide by parent.
  Center,
  /// The children are aligned to the bottom edge of the box provided by parent.
  Bottom,
  /// Require the children to fill the whole box in y-axis. This causes the
  /// constraints passed to the children to be tight.
  Stretch,
}

/// A virtual widget that horizontally positions its child based on parent
/// constraints.
///
/// ## Layout Behavior
///
/// As we don't know the parent size during layout, the alignment container
/// width is determined by the following priority rules:
///
/// 1. Use the maximum constraint width if it's finite.
/// 2. Fall back to the clamped child width (constrained by min/max limits) .
///
/// ## Implementation Notes
///
/// - Parent precedence: Final x-position may be overridden by parent layout
///   logic
/// - Constraint relaxation: This widget removes width constraints during child
///   layout to ensure proper alignment calculation
///
/// ## Usage Guidelines
///
/// For reliable alignment:
///
/// - Use within parents with fixed width
/// - Avoid combining with parents that perform custom x-axis positioning
#[derive(Default)]
pub struct HAlignWidget {
  /// Configuration for horizontal alignment strategy
  ///
  /// See [`HAlign`] documentation for available alignment options and their
  /// behavior in different layout scenarios.
  pub h_align: HAlign,
}

/// A virtual widget that vertically positions its child based on parent
/// constraints.
///
/// This widget is similar to [`HAlignWidget`], but aligns children in y-axis.
/// See [`HAlignWidget`] for more details.
#[derive(Default)]
pub struct VAlignWidget {
  /// Configuration for vertical alignment strategy
  ///
  /// See [`VAlign`] documentation for available alignment options and their
  /// behavior in different layout scenarios.
  pub v_align: VAlign,
}

/// Macro used to generate a function widget using the `HAlignWidget` as the
/// root widget.
#[macro_export]
macro_rules! h_align_widget {
  ($($t: tt)*) => { fn_widget! { @HAlignWidget { $($t)* } } };
}

/// Macro used to generate a function widget using the `VAlignWidget` as the
/// root widget.
#[macro_export]
macro_rules! v_align_widget {
  ($($t: tt)*) => { fn_widget! { @VAlignWidget { $($t)* } } };
}

impl Declare for HAlignWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl Declare for VAlignWidget {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(HAlignWidget);
impl_compose_child_for_wrap_render!(VAlignWidget);

impl WrapRender for HAlignWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.max.width;
    let host_clamp = if self.h_align == HAlign::Stretch && max.is_finite() {
      clamp.with_fixed_width(max)
    } else {
      clamp.free_width()
    };
    let size = host.perform_layout(host_clamp, ctx);

    // Get the alignment container, see the [`HAlignWidget`] documentation.
    let container = clamp.container_width(size.width);
    let x = Align::align_value(self.h_align.into(), size.width, container);
    let pos = ctx.box_pos().unwrap_or_default();
    ctx.update_position(ctx.widget_id(), Point::new(x, pos.y));

    // Don't clamp, because we free the width to get proper alignment
    size
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

impl WrapRender for VAlignWidget {
  fn perform_layout(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut LayoutCtx) -> Size {
    let max = clamp.max.height;
    let host_clamp = if self.v_align == VAlign::Stretch && max.is_finite() {
      clamp.with_fixed_height(max)
    } else {
      clamp.free_height()
    };
    let size = host.perform_layout(host_clamp, ctx);

    // Get the alignment container, see the [`VAlignWidget`] documentation.
    let container = clamp.container_height(size.height);
    let y = Align::align_value(self.v_align.into(), size.height, container);
    let pos = ctx.box_pos().unwrap_or_default();
    ctx.update_position(ctx.widget_id(), Point::new(pos.x, y));

    // Don't clamp, because we free the height to get proper alignment
    size
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }
}

impl Align {
  pub fn align_value(self, child_size: f32, box_size: f32) -> f32 {
    if !box_size.is_finite() {
      return 0.;
    }

    match self {
      Align::Center => (box_size - child_size) / 2.,
      Align::End => box_size - child_size,
      _ => 0.,
    }
  }
}

impl From<HAlign> for Align {
  fn from(h: HAlign) -> Self {
    match h {
      HAlign::Left => Align::Start,
      HAlign::Center => Align::Center,
      HAlign::Right => Align::End,
      HAlign::Stretch => Align::Stretch,
    }
  }
}

impl From<VAlign> for Align {
  fn from(h: VAlign) -> Self {
    match h {
      VAlign::Top => Align::Start,
      VAlign::Center => Align::Center,
      VAlign::Bottom => Align::End,
      VAlign::Stretch => Align::Stretch,
    }
  }
}

impl From<Align> for HAlign {
  fn from(h: Align) -> Self {
    match h {
      Align::Start => HAlign::Left,
      Align::Center => HAlign::Center,
      Align::End => HAlign::Right,
      Align::Stretch => HAlign::Stretch,
    }
  }
}

impl From<Align> for VAlign {
  fn from(h: Align) -> Self {
    match h {
      Align::Start => VAlign::Top,
      Align::Center => VAlign::Center,
      Align::End => VAlign::Bottom,
      Align::Stretch => VAlign::Stretch,
    }
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;
  const CHILD_SIZE: Size = Size::new(10., 10.);
  const WND_SIZE: Size = Size::new(100., 100.);

  fn h_align(h_align: HAlign) -> GenWidget {
    fn_widget! {
      @HAlignWidget {
        h_align,
        @MockBox { size: CHILD_SIZE }
      }
    }
    .r_into()
  }

  // Edge case: box_size is infinity
  #[test]
  fn align_value_infinity() {
    let align = Align::Center;
    assert_eq!(align.align_value(10., f32::INFINITY), 0.);
  }

  // Edge case: box_size is NaN
  #[test]
  fn align_value_nan() {
    let align = Align::Center;
    assert_eq!(align.align_value(10., f32::NAN), 0.);
  }

  // Edge case: child_size > box_size
  #[test]
  fn align_value_child_larger_than_box() {
    let align = Align::Center;
    assert_eq!(align.align_value(20., 10.), -5.);
  }

  // Edge case: child_size == box_size
  #[test]
  fn align_value_child_equal_to_box() {
    let align = Align::Center;
    assert_eq!(align.align_value(10., 10.), 0.);
  }

  widget_layout_test!(
    left_align,
    WidgetTester::new(h_align(HAlign::Left)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_x(0.)
  );

  widget_layout_test!(
    h_center_align,
    WidgetTester::new(h_align(HAlign::Center)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_x(45.)
  );

  widget_layout_test!(
    right_align,
    WidgetTester::new(h_align(HAlign::Right)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_x(90.)
  );

  widget_layout_test!(
    h_stretch_algin,
    WidgetTester::new(h_align(HAlign::Stretch)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(Size::new(WND_SIZE.width, 10.))
      .with_x(0.)
  );

  fn v_align(v_align: VAlign) -> GenWidget {
    fn_widget! {
      @VAlignWidget {
        v_align,
        @MockBox { size: CHILD_SIZE }
      }
    }
    .r_into()
  }

  widget_layout_test!(
    top_align,
    WidgetTester::new(v_align(VAlign::Top)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_y(0.)
  );

  widget_layout_test!(
    v_center_align,
    WidgetTester::new(v_align(VAlign::Center)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_y(45.)
  );

  widget_layout_test!(
    bottom_align,
    WidgetTester::new(v_align(VAlign::Bottom)).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_y(90.)
  );

  widget_layout_test!(
    v_stretch_align,
    WidgetTester::new(v_align(VAlign::Stretch)).with_wnd_size(WND_SIZE),
    LayoutCase::default().with_size(Size::new(10., WND_SIZE.height))
  );

  fn all_align() -> GenWidget {
    fn_widget! {
        @MockBox {
          h_align: HAlign::Center,
          v_align: VAlign::Center,
          size: CHILD_SIZE
        }
    }
    .r_into()
  }

  widget_layout_test!(
    all_align,
    WidgetTester::new(all_align()).with_wnd_size(WND_SIZE),
    LayoutCase::default()
      .with_size(CHILD_SIZE)
      .with_x(45.)
      .with_y(45.)
  );
}
