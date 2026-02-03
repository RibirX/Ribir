use crate::{prelude::*, wrap_render::*};

/// A wrapper that applies additional constraint clamping to its child.
///
/// This is a built-in `FatObj` field. Setting the `clamp` field attaches a
/// `ConstrainedBox` which constrains the child's min/max sizes.
///
/// # Example
///
/// Constrain a container to a maximum width of 100.
///
/// ```rust
/// use ribir::prelude::*;
///
/// container! {
///   size: Size::new(200., 50.), // This will be constrained to width 100.
///   background: Color::RED,
///   clamp: BoxClamp::max_width(100.),
/// };
/// ```
#[derive(Clone, Default)]
pub struct ConstrainedBox {
  pub clamp: BoxClamp,
}

impl Declare for ConstrainedBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl_compose_child_for_wrap_render!(ConstrainedBox);

impl WrapRender for ConstrainedBox {
  fn measure(&self, clamp: BoxClamp, host: &dyn Render, ctx: &mut MeasureCtx) -> Size {
    let max = clamp.clamp(self.clamp.max);
    let min = clamp.clamp(self.clamp.min);
    host.measure(BoxClamp { min, max }, ctx)
  }

  fn size_affected_by_child(&self, host: &dyn Render) -> bool {
    let is_fixed = self.clamp.min == self.clamp.max;
    if is_fixed { false } else { host.size_affected_by_child() }
  }

  #[inline]
  fn wrapper_dirty_phase(&self) -> DirtyPhase { DirtyPhase::Layout }

  #[cfg(feature = "debug")]
  fn debug_type(&self) -> Option<&'static str> { Some("constrainedBox") }

  #[cfg(feature = "debug")]
  fn debug_properties(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
      "min": { "width": self.clamp.min.width, "height": self.clamp.min.height },
      "max": { "width": self.clamp.max.width, "height": self.clamp.max.height }
    }))
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    outside_fixed_clamp,
    WidgetTester::new(fn_widget! {
      @ConstrainedBox {
        clamp: BoxClamp::fixed_size(Size::new(50., 50.)),
        @Void {
          clamp: BoxClamp::fixed_size(Size::new(40., 40.))
        }
      }
    }),
    LayoutCase::new(&[0]).with_size(Size::new(50., 50.))
  );

  widget_layout_test!(
    min_width_limit,
    WidgetTester::new(fn_widget! {
      @Void {
        min_width: 100.,
        min_height: 100.,
      }
    }),
    LayoutCase::new(&[0]).with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    max_width_limit,
    WidgetTester::new(fn_widget! {
      @MockBox {
        size: Size::new(200., 200.),
        max_width: 100.,
        max_height: 100.,
      }
    }),
    LayoutCase::new(&[0]).with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    min_max_size,
    WidgetTester::new(fn_widget! {
      @MockBox {
        size: Size::new(50., 50.),
        min_size: Size::new(100., 100.),
        max_size: Size::new(150., 150.),
      }
    }),
    LayoutCase::new(&[0]).with_size(Size::new(100., 100.))
  );
}
