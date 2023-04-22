use ribir_core::{impl_query_self_only, prelude::*};

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(SingleChild, Declare, Clone)]
pub struct SizedBox {
  pub size: Size,
}

impl Render for SizedBox {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.perform_single_child_layout(BoxClamp { min: self.size, max: self.size });
    self.size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let rect = Rect::from_size(ctx.box_rect().unwrap().size);
    let path = Path::rect(&rect);
    ctx.painter().clip(path);
  }
}

impl Query for SizedBox {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test::*;

  #[test]
  fn fix_size() {
    let size: Size = Size::new(100., 100.);
    let w = widget! {
      SizedBox {
        size,
        Text { text: "" }
      }
    };

    expect_layout_result(
      w,
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(size),
      }],
    );
  }

  #[test]
  fn shrink_size() {
    let w = widget! {
      SizedBox {
        size: ZERO_SIZE,
        Text { text: "" }
      }
    };

    expect_layout_result(
      w,
      None,
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(ZERO_SIZE),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::from_size(ZERO_SIZE),
        },
      ],
    );
  }

  #[test]
  fn expanded_size() {
    let wnd_size = Size::new(500., 500.);
    let expand_box = widget! {
      SizedBox {
        size: INFINITY_SIZE,
        Text { text: "" }
      }
    };

    expect_layout_result(
      expand_box,
      Some(wnd_size),
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(wnd_size),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::from_size(INFINITY_SIZE),
        },
      ],
    );
  }

  #[test]
  fn empty_box() {
    let size = Size::new(10., 10.);
    expect_layout_result(
      SizedBox { size }.into_widget(),
      None,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect::from_size(size),
      }],
    );
  }
}
