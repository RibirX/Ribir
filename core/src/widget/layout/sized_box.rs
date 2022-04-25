use crate::prelude::*;

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(SingleChildWidget, Declare, Clone)]
pub struct SizedBox {
  pub size: Size,
}

impl SizedBox {
  /// Creates a box that will become as large as its parent allows.
  #[inline]
  pub fn expanded_size() -> Size {
    const INFINITY: f32 = f32::INFINITY;
    Size::new(INFINITY, INFINITY)
  }

  /// Creates a box that will become as small as its parent allows.
  #[inline]
  pub fn shrink_size() -> Size { Size::zero() }
}

impl Render for SizedBox {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let size = clamp.clamp(self.size);
    if let Some(child) = ctx.single_child() {
      ctx.perform_render_child_layout(child, BoxClamp { min: size, max: size });
    }
    size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn fix_size() {
    const SIZE: Size = Size::new(100., 100.);
    struct T;
    impl Compose for T {
      #[widget]
      fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        widget! {
          declare SizedBox {
            size:SIZE,
            Text { text: "" }
          }
        }
      }
    }

    let (rect, child) = widget_and_its_children_box_rect(T.box_it(), Size::new(500., 500.));
    assert_eq!(rect.size, SIZE);
    assert_eq!(child, vec![Rect::from_size(SIZE)]);
  }

  #[test]
  fn shrink_size() {
    struct Shrink;

    impl Compose for Shrink {
      #[widget]
      fn compose(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        widget! {
          declare SizedBox {
            size: SizedBox::shrink_size(),
            Text { text: "" }
          }
        }
      }
    }

    let (rect, child) = widget_and_its_children_box_rect(Shrink.box_it(), Size::new(500., 500.));

    assert_eq!(rect.size, Size::zero());
    assert_eq!(child, vec![Rect::zero()]);
  }

  #[test]
  fn expanded_size() {
    let wnd_size = Size::new(500., 500.);
    let expand_box = SizedBox { size: SizedBox::expanded_size() }
      .have_child(Text {
        text: "".into(),
        style: <_>::default(),
      })
      .box_it();

    let (rect, child) = widget_and_its_children_box_rect(expand_box, Size::new(500., 500.));

    assert_eq!(rect.size, wnd_size);
    assert_eq!(child, vec![Rect::from_size(wnd_size)]);
  }

  #[test]
  fn empty_box() {
    let size = Size::new(10., 10.);
    let empty_box = SizedBox { size };
    let (rect, child) = widget_and_its_children_box_rect(empty_box.box_it(), Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![]);
  }
}
