use crate::{impl_query_self_only, prelude::*};

/// Widget with fixed size as a container for its child.
#[derive(Declare, SingleChild, Clone)]
pub struct Container {
  pub size: Size,
}

impl Render for Container {
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if let Some(mut l) = ctx.single_child_layouter() {
      clamp.max = clamp.max.min(self.size);
      clamp.min = clamp.max.min(clamp.min);
      l.perform_widget_layout(clamp);
    };
    self.size
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }
}

impl Query for Container {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    let size = Size::new(100., 100.);

    expect_layout_result(
      widget! { Container { size }},
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(size),
      }],
    );
  }
}
