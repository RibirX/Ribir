use ribir_core::{impl_query_self_only, prelude::*};

/// Enum to describe which axis will expand of constraints on its child, use by
/// `ExpandBox`.
#[derive(Default, Clone, Copy)]
pub enum ExpandDir {
  X,
  Y,
  #[default]
  Both,
}

/// A box that will expand the specify axis to the parent's max clamp size
#[derive(SingleChild, Declare, Clone)]
pub struct ExpandBox {
  dir: ExpandDir,
}

impl Render for ExpandBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let mut size = Size::zero();
    if let Some(child) = ctx.single_child() {
      size = ctx.perform_child_layout(child, clamp);
    }
    match self.dir {
      ExpandDir::X => size.width = clamp.max.width,
      ExpandDir::Y => size.height = clamp.max.height,
      ExpandDir::Both => size = clamp.max,
    }
    size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for ExpandBox {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test::*;

  #[test]
  fn expand_one_axis() {
    let w = widget! {
      Container {
        size: Size::new(256., 50.),
        ExpandBox {
          dir: ExpandDir::X,
          Container {
            size: Size::new(128., 20.),
          }
        }
      }
    };
    expect_layout_result_with_theme(
      w,
      None,
      material::purple::light(),
      &[LayoutTestItem {
        path: &[0, 0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(256.),
          height: Some(20.),
        },
      }],
    );
  }

  #[test]
  fn expand_both() {
    let w = widget! {
      Container {
        size: Size::new(256., 50.),
        ExpandBox {
          dir: ExpandDir::Both,
          Container {
            size: Size::new(128., 20.),
          }
        }
      }
    };
    expect_layout_result_with_theme(
      w,
      None,
      material::purple::light(),
      &[LayoutTestItem {
        path: &[0, 0],
        expect: ExpectRect {
          x: Some(0.),
          y: Some(0.),
          width: Some(256.),
          height: Some(50.),
        },
      }],
    );
  }
}
