use crate::prelude::*;

#[derive(Declare, Query, SingleChild)]
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
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if self.clamp_dim.contains(ClampDim::MIN_SIZE) {
      match self.dir {
        UnconstrainedDir::X => {
          clamp.min.width = 0.;
        }
        UnconstrainedDir::Y => {
          clamp.min.height = 0.;
        }
        UnconstrainedDir::Both => clamp = clamp.loose(),
      };
    }
    if self.clamp_dim.contains(ClampDim::MAX_SIZE) {
      match self.dir {
        UnconstrainedDir::X => {
          clamp.max.width = f32::INFINITY;
        }
        UnconstrainedDir::Y => {
          clamp.max.height = f32::INFINITY;
        }
        UnconstrainedDir::Both => clamp = clamp.expand(),
      };
    }

    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  fn smoke() -> impl WidgetBuilder {
    let size = Size::new(200., 200.);
    fn_widget! {
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
    }
  }
  widget_layout_test!(
    smoke,
    wnd_size = Size::new(100., 100.),
    { path = [0, 0, 0],width == 200., height == 200.,}
    { path = [0, 1, 0],width == 200., height == 100.,}
    { path = [0, 2, 0],width == 100., height == 200.,}
  );
}
