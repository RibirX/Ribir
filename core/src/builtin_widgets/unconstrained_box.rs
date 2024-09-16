use crate::prelude::*;

#[derive(Declare, SingleChild)]
/// A widget that imposes no constraints on its child, allowing it to layout and
/// display as its "natural" size. Its size is equal to its child then clamp by
/// parent.
pub struct UnconstrainedBox {
  #[declare(default)]
  pub dir: UnconstrainedDir,

  #[declare(default)]
  pub clamp_dim: ClampDim,
}

/// Enum to describe which axis will imposes no constraints on its child, use by
/// `UnConstrainedBox`.
#[derive(Default, Clone, Copy)]
pub enum UnconstrainedDir {
  X,
  Y,
  #[default]
  Both,
}

bitflags! {
  /// Enum to describe which box clamp dim will imposes no constraints on its
  /// child, use by `UnConstrainedBox`.
  ///
  #[derive(Clone, Copy, Eq, PartialEq)]
  pub struct ClampDim: u8 {
    const MIN_SIZE = 0x01;
    const MAX_SIZE = 0x02;
    const Both = Self::MIN_SIZE.bits() | Self::MAX_SIZE.bits();
  }
}

impl Default for ClampDim {
  fn default() -> Self { ClampDim::Both }
}

impl Render for UnconstrainedBox {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut child_clamp = clamp;
    if self.clamp_dim.contains(ClampDim::MIN_SIZE) {
      match self.dir {
        UnconstrainedDir::X => child_clamp.min.width = 0.,
        UnconstrainedDir::Y => child_clamp.min.height = 0.,
        UnconstrainedDir::Both => child_clamp = child_clamp.loose(),
      };
    }
    if self.clamp_dim.contains(ClampDim::MAX_SIZE) {
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
