use crate::{impl_query_self_only, prelude::*};

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(SingleChild, Declare, Clone)]
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
    if let Some(child) = ctx.single_child() {
      let size = clamp.clamp(self.size);
      ctx.perform_child_layout(child, BoxClamp { min: size, max: size });
    }
    self.size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for SizedBox {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn fix_size() {
    let size: Size = Size::new(100., 100.);
    let w = widget! {
      SizedBox {
        size,
        Text { text: "" }
      }
    };

    let (rect, child) = widget_and_its_children_box_rect(w, Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![Rect::from_size(size)]);
  }

  #[test]
  fn shrink_size() {
    let w = widget! {
      SizedBox {
        size: SizedBox::shrink_size(),
        Text { text: "" }
      }
    };
    let (rect, child) = widget_and_its_children_box_rect(w, Size::new(500., 500.));

    assert_eq!(rect.size, Size::zero());
    assert_eq!(child, vec![Rect::zero()]);
  }

  #[test]
  fn expanded_size() {
    let wnd_size = Size::new(500., 500.);
    let expand_box = widget! {
      SizedBox {
        size: SizedBox::expanded_size(),
        Text { text: "" }
      }
    };
    let (rect, child) = widget_and_its_children_box_rect(expand_box, Size::new(500., 500.));

    assert_eq!(rect.size, wnd_size);
    assert_eq!(child, vec![Rect::from_size(wnd_size)]);
  }

  #[test]
  fn empty_box() {
    let size = Size::new(10., 10.);
    let empty_box = SizedBox { size };
    let (rect, child) =
      widget_and_its_children_box_rect(empty_box.into_widget(), Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![]);
  }
}
