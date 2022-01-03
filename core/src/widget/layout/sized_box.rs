use crate::prelude::*;

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[stateful]
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

impl RenderWidget for SizedBox {
  type RO = Self;
  #[inline]
  fn create_render_object(&self) -> Self::RO { self.clone() }

  fn update_render_object(&self, object: &mut Self::RO, ctx: &mut UpdateCtx) {
    if self.size != object.size {
      object.size = self.size;
      ctx.mark_needs_layout();
    }
  }
}

impl RenderObject for SizedBox {
  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let size = clamp.clamp(self.size);
    if let Some(child) = ctx.single_child() {
      ctx.perform_child_layout(child, BoxClamp { min: size, max: size });
    }
    size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {
    // nothing to paint, just a layout widget.
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn fix_size() {
    let size = Size::new(100., 100.);
    let sized_box = declare! {
      SizedBox {
        size,
        Text { text: "", style: <_>::default() }
      }
    };

    let (rect, child) = widget_and_its_children_box_rect(sized_box.box_it(), Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![Rect::from_size(size)]);
  }

  #[test]
  fn shrink_size() {
    let shrink = declare! {
      SizedBox {
        size: SizedBox::shrink_size(),
        Text { text: "", style: <_>::default()}
      }
    };
    let (rect, child) = widget_and_its_children_box_rect(shrink.box_it(), Size::new(500., 500.));

    assert_eq!(rect.size, Size::zero());
    assert_eq!(child, vec![Rect::zero()]);
  }

  #[test]
  fn expanded_size() {
    let wnd_size = Size::new(500., 500.);
    let expand_box = declare! {
      SizedBox {
        size: SizedBox::expanded_size(),
        Text { text:"" , style: <_>::default(),}
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
    let (rect, child) = widget_and_its_children_box_rect(empty_box.box_it(), Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![]);
  }
}
