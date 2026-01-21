use ribir_core::prelude::*;

use crate::prelude::{NoAffectedParentSize, no_affected_parent_size};
/// A widget that overlaps its children, allowing for flexible layout
/// management.
///
/// The `Stack` widget manages its children in two phases:
/// 1. **Normal children Layout**: Children not wrapped with [`InParentLayout`]
///    are laid out first. These children determine the size of the `Stack` and
///    are aligned to the top-left.
/// 2. **In-Parent Layout**: Other children are laid out afterward, using the
///    `Stack`'s size as constraints, enabling them to be positioned or aligned
///    relative to the `Stack`.
#[derive(MultiChild, Declare, Default)]
pub struct Stack {
  #[declare(default)]
  fit: StackFit,
}

/// A widget that indicates to its parent that it should perform layout within
/// the parent's bounds, and the layout changes to the subtree do not
/// trigger stack to re-layout.
///
/// This widget is used to control the layout behavior of its child within a
/// `Stack`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[declare(simple)]
pub struct InParentLayout;

/// Determines how the constraints are passed to the children of a [`Stack`].
///
/// This enum controls the layout behavior of the `Stack`'s children based on
/// the constraints provided by the parent widget.
#[derive(Default)]
pub enum StackFit {
  /// Loosens the constraints passed to the stack from its parent.
  ///
  /// For example, if the stack has constraints of 350x600, the children can
  /// have any width from 0 to 350 and any height from 0 to 600.
  #[default]
  Loose,

  /// Tightens the constraints to the maximum size allowed.
  ///
  /// For example, if the stack has loose constraints with a width range of
  /// 10 to 100 and a height range of 0 to 600, the children will be sized to
  /// 100x600.
  Expand,

  /// Passes the constraints through without modification.
  Passthrough,
}

impl Render for Stack {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    // Determine appropriate clamp based on stack fit
    let stack_clamp = match self.fit {
      StackFit::Loose => clamp.loose(),
      StackFit::Expand => {
        let max = clamp.max;
        let mut clamp = clamp;
        if max.width.is_finite() {
          clamp = clamp.with_fixed_width(max.width);
        }
        if max.height.is_finite() {
          clamp = clamp.with_fixed_height(max.height);
        }
        clamp
      }
      _ => clamp,
    };

    // Split children into those that need parent-sized layout and others
    let (ctx, children) = ctx.split_children();
    let (in_parents, regulars): (Vec<_>, _) = children.into_iter().partition(|child| {
      ctx
        .query_of_widget::<InParentLayout>(*child)
        .is_some()
    });

    // Layout regular children first
    let stack_size = regulars
      .into_iter()
      .fold(ZERO_SIZE, |max_size, child| {
        let child_size = ctx.layout_child(child, stack_clamp);
        max_size.max(child_size)
      });

    // When we have valid size, layout in-parent children with parent-relative
    // constraints
    let stack_size = clamp.clamp(stack_size);
    if !in_parents.is_empty() {
      let parent_relative_clamp = BoxClamp::max_size(stack_size);
      in_parents.into_iter().for_each(|child| {
        ctx.layout_child(child, parent_relative_clamp);
      });
    }
    stack_size
  }
}

impl<'c> ComposeChild<'c> for InParentLayout {
  type Child = Widget<'c>;

  fn compose_child(_: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    no_affected_parent_size! { @ { child } }
      .into_widget()
      .attach_data(Box::new(Queryable(InParentLayout)))
  }
}
#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;

  const ONE: Size = Size::new(1., 1.);
  const FIVE: Size = Size::new(5., 5.);
  const TEN: Size = Size::new(10., 10.);

  widget_layout_test!(
    smoke,
    WidgetTester::new(stack! {
      @Container { size: ONE }
      @Container { size: FIVE }
    }),
    LayoutCase::default().with_size(FIVE)
  );

  widget_layout_test!(
    stack_fit_loose,
    WidgetTester::new(stack! {
      clamp: BoxClamp::min_size(TEN),
      fit: StackFit::Loose,
      @Container { size: ONE }
      @Container { size: FIVE }
    }),
    LayoutCase::default().with_size(TEN),
    LayoutCase::new(&[0, 0]).with_size(ONE),
    LayoutCase::new(&[0, 1]).with_size(FIVE)
  );

  widget_layout_test!(
    stack_fit_expand,
    WidgetTester::new(stack! {
      clamp: BoxClamp::max_size(TEN),
      fit: StackFit::Expand,
      @Container { size: ONE }
      @Container { size: FIVE }
    }),
    LayoutCase::default().with_size(TEN),
    LayoutCase::new(&[0, 0]).with_size(TEN),
    LayoutCase::new(&[0, 1]).with_size(TEN)
  );

  widget_layout_test!(
    stack_fit_passthrough,
    WidgetTester::new(stack! {
      clamp: BoxClamp::fixed_size(FIVE),
      fit: StackFit::Passthrough,
      @Container { size: ONE }
      @Container { size: TEN }
    }),
    LayoutCase::default().with_size(FIVE)
  );

  widget_layout_test!(
    in_parent_layout,
    WidgetTester::new(stack! {
      @Container { size: TEN }
      @InParentLayout {
        @Container { x: AnchorX::right(), size: ONE }
      }
    }),
    LayoutCase::default().with_size(TEN),
    LayoutCase::new(&[0, 1, 0])
      .with_size(ONE)
      .with_x(9.)
  );

  widget_layout_test!(
    empty_stack,
    WidgetTester::new(stack! {}),
    LayoutCase::default().with_size(ZERO_SIZE)
  );
}
