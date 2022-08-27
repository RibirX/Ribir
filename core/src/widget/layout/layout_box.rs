use crate::prelude::*;

/// Widget let user to access the layout result of its child.
#[derive(Declare)]
pub struct LayoutBox {
  #[declare(skip)]
  /// the rect box of its child and the coordinate is relative to its parent.
  rect: Rect,
}

impl ComposeSingleChild for LayoutBox {
  fn compose_single_child(this: StateWidget<Self>, child: Widget, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      ExprWidget {
        expr: child,
        on_performed_layout: move |ctx| this.rect = ctx.box_rect().unwrap()
      }
    }
  }
}

impl LayoutBox {
  #[inline]
  pub fn box_rect(&self) -> Rect { self.rect }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::{expect_layout_result, ExpectRect, LayoutTestItem};

  #[test]
  fn smoke() {
    expect_layout_result(
      Size::new(500., 500.),
      widget! {
        Row {
          LayoutBox {
            id: layout_box,
            SizedBox { size: Size::new(100., 200.) }
          }
          SizedBox { size: layout_box.rect.size }
        }
      },
      &[
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
