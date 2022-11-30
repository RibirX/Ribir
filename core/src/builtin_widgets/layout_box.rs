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
      states { this: this.into_stateful() }
      DynWidget {
        dyns: child,
        performed_layout: move |ctx| {
          let new_rect = ctx.box_rect().unwrap();
          if this.rect != new_rect {
            this.silent().rect = new_rect;
          }
        }
      }
    }
  }
}

impl LayoutBox {
  /// return the rect after layout of the widget
  #[inline]
  pub fn layout_rect(&self) -> Rect { self.rect }

  /// return the position relative to parent after layout of the widget
  #[inline]
  pub fn layout_pos(&self) -> Point { self.rect.origin }

  /// return the size after layout of the widget
  #[inline]
  pub fn layout_size(&self) -> Size { self.rect.size }

  /// return the left position relative parent after layout of the widget
  #[inline]
  pub fn layout_left(&self) -> f32 { self.rect.min_x() }

  /// return the top position relative parent after layout of the widget
  #[inline]
  pub fn layout_top(&self) -> f32 { self.rect.min_y() }

  /// return the width after layout of the widget
  #[inline]
  pub fn layout_width(&self) -> f32 { self.rect.width() }

  /// return the height after layout of the widget
  #[inline]
  pub fn layout_height(&self) -> f32 { self.rect.height() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;

  #[test]
  fn smoke() {
    expect_layout_result(
      widget! {
        MockMulti {
          LayoutBox {
            id: layout_box,
            MockBox { size: Size::new(100., 200.) }
          }
          MockBox { size: layout_box.rect.size }
        }
      },
      None,
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
