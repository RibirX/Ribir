use crate::prelude::*;

#[derive(Declare, SingleChild)]
/// A widget that imposes no constraints on its child, allowing it to layout and
/// display as its "natural" size. Its size is equal to its child then clamp by
/// parent.
pub struct UnconstrainedBox {
  #[declare(default)]
  pub dir: UnconstrainedDir,
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

impl Render for UnconstrainedBox {
  #[inline]
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    match self.dir {
      UnconstrainedDir::X => {
        clamp.min.width = 0.;
        clamp.max.width = f32::INFINITY;
      }
      UnconstrainedDir::Y => {
        clamp.min.height = 0.;
        clamp.max.height = f32::INFINITY;
      }
      UnconstrainedDir::Both => clamp = clamp.expand().loose(),
    };
    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for UnconstrainedBox {
  crate::impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    let size = Size::new(200., 200.);
    let w = widget! {
      MockMulti {
        UnconstrainedBox {
          Container { size}
        }
        UnconstrainedBox {
          dir: UnconstrainedDir::X,
          Container { size }
        }
        UnconstrainedBox {
          dir: UnconstrainedDir::Y,
          Container { size }
        }
      }
    };

    expect_layout_result(
      w,
      Some(Size::new(100., 100.)),
      &[
        LayoutTestItem {
          path: &[0, 0, 0],
          expect: ExpectRect {
            width: Some(200.),
            height: Some(200.),
            ..<_>::default()
          },
        },
        LayoutTestItem {
          path: &[0, 1, 0],
          expect: ExpectRect {
            width: Some(200.),
            height: Some(100.),
            ..<_>::default()
          },
        },
        LayoutTestItem {
          path: &[0, 2, 0],
          expect: ExpectRect {
            width: Some(100.),
            height: Some(200.),
            ..<_>::default()
          },
        },
      ],
    );
  }
}
