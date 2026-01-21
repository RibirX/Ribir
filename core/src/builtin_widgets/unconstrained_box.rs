use crate::prelude::*;

#[derive(Declare, SingleChild, Default)]
/// A widget that imposes no constraints on its child, allowing it to layout and
/// display as its "natural" size. Its size is equal to its child then clamp by
/// parent.
///
/// # Example
///
/// ```rust no_run
/// use ribir::prelude::*;
///
/// fn_widget! {
///   @Container {
///     size: Size::new(100., 100.),
///     @UnconstrainedBox {
///       @Container {
///         size: Size::new(200., 200.),
///         background: Color::RED,
///       }
///     }
///   }
/// };
/// ```
pub struct UnconstrainedBox {
  #[declare(default)]
  pub dir: UnconstrainedDir,

  #[declare(default)]
  pub clamp_dim: ClampDim,
}

/// Enum to describe which axis will imposes no constraints on its child, use by
/// `UnConstrainedBox`.
#[derive(Default, Clone, Copy, Eq, PartialEq)]
pub enum UnconstrainedDir {
  X,
  Y,
  #[default]
  Both,
}

/// Enum to describe which box clamp dim will imposes no constraints on its
/// child, use by `UnConstrainedBox`.
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub enum ClampDim {
  Min,
  Max,
  #[default]
  Both,
}

impl Render for UnconstrainedBox {
  #[inline]
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    let mut child_clamp = clamp;
    if self.clamp_dim != ClampDim::Max {
      match self.dir {
        UnconstrainedDir::X => child_clamp.min.width = 0.,
        UnconstrainedDir::Y => child_clamp.min.height = 0.,
        UnconstrainedDir::Both => child_clamp = child_clamp.loose(),
      };
    }
    if self.clamp_dim != ClampDim::Min {
      match self.dir {
        UnconstrainedDir::X => child_clamp.max.width = f32::INFINITY,
        UnconstrainedDir::Y => child_clamp.max.height = f32::INFINITY,
        UnconstrainedDir::Both => child_clamp = child_clamp.expand(),
      };
    }
    let size = ctx.assert_perform_single_child_layout(child_clamp);
    clamp.clamp(size)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      let size = Size::new(200., 200.);
      @MockMulti {
        @UnconstrainedBox {
          @MockBox { size}
        }
        @UnconstrainedBox {
          dir: UnconstrainedDir::X,
          @MockBox { size }
        }
        @UnconstrainedBox {
          dir: UnconstrainedDir::Y,
          @MockBox { size }
        }
      }
    })
    .with_wnd_size(Size::new(100., 100.)),
    LayoutCase::new(&[0, 0, 0]).with_size(Size::new(200., 200.)),
    LayoutCase::new(&[0, 1, 0]).with_size(Size::new(200., 100.)),
    LayoutCase::new(&[0, 2, 0]).with_size(Size::new(100., 200.))
  );
}
