use crate::prelude::*;

/// Widget let user to access the layout result of its child.
#[derive(Declare)]
pub struct LayoutBox {
  #[declare(skip)]
  /// the rect box of its child and the coordinate is relative to its parent.
  rect: Rect,
}

impl ComposeChild for LayoutBox {
  type Child = Widget;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      ExprWidget {
        expr: child,
        performed_layout: move |ctx| this.silent().rect = ctx.box_rect().unwrap()
      }
    }
  }
}

impl LayoutBox {
  #[inline]
  pub fn box_rect(&self) -> Rect { self.rect }
}

impl std::ops::Deref for LayoutBox {
  type Target = Rect;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.rect }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::{expect_layout_result, ExpectRect, LayoutTestItem};

  #[test]
  fn smoke() {
    expect_layout_result(
      widget! {
        Row {
          LayoutBox {
            id: layout_box,
            SizedBox { size: Size::new(100., 200.) }
          }
          SizedBox { size: layout_box.rect.size }
        }
      },
      None,
      None,
        & [
          LayoutTestItem {
            path: &[0],
            expect: ExpectRect {
              width: Some(200.),
              height: Some(200.),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 0],
            expect: ExpectRect {
              width: Some(100.),
              height: Some(200.),
              ..<_>::default()
            },
          },
          LayoutTestItem {
            path: &[0, 1],
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
