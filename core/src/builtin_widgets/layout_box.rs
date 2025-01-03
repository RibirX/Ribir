use crate::prelude::*;

/// Widget let user to access the layout result of its child.
#[derive(Default)]
pub struct LayoutBox {
  /// the rect box of its child and the coordinate is relative to its parent.
  rect: Rect,
}

impl Declare for LayoutBox {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for LayoutBox {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      FatObj::new(child).on_performed_layout(move |e| {
        let new_rect = e.box_rect().unwrap();
        if $this.rect != new_rect {
          $this.silent().rect = new_rect;
        }
      })
    }
    .into_widget()
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
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! {
      let mut first_box = @MockBox { size: Size::new(100., 200.) };
      let second_box = @MockBox { size: pipe!($first_box.layout_size()) };
      @MockMulti {
        @ { [first_box, second_box ] }
      }
    }),
    LayoutCase::default().with_size(Size::new(200., 200.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(100., 200.)),
    LayoutCase::new(&[0, 1]).with_size(Size::new(100., 200.))
  );
}
