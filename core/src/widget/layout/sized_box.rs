use crate::prelude::*;
pub use smallvec::{smallvec, SmallVec};

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[stateful]
#[derive(SingleChildWidget, AttachAttr)]
pub struct SizedBox {
  #[state]
  pub size: Size,
}

impl SizedBox {
  /// Creates a box with the specified size.
  #[inline]
  pub fn from_size(size: Size) -> SizedBox { Self { size } }

  /// Creates a box that will become as large as its parent allows.
  #[inline]
  pub fn expanded() -> SizedBox {
    const INFINITY: f32 = f32::INFINITY;
    Self { size: Size::new(INFINITY, INFINITY) }
  }

  /// Creates a box that will become as small as its parent allows.
  #[inline]
  pub fn shrink() -> Self { Self { size: Size::zero() } }
}

impl RenderWidget for SizedBox {
  type RO = SizedBoxState;
  #[inline]
  fn create_render_object(&self) -> Self::RO { SizedBoxState { size: self.size } }
}

impl RenderObject for SizedBoxState {
  type States = SizedBoxState;

  #[inline]
  fn update(&mut self, states: Self::States, _: &mut UpdateCtx) { self.size = states.size; }

  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let size = clamp.clamp(self.size);
    let mut child_iter = ctx.children();
    let child = child_iter.next();
    debug_assert!(child_iter.next().is_none());
    if let Some(mut child_ctx) = child {
      child_ctx.perform_layout(BoxClamp { min: size, max: size });
    }
    size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {
    // nothing to paint, just a layout widget.
  }

  #[inline]
  fn get_states(&self) -> &Self::States { self }
}

impl StatePartialEq<Self> for Size {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self == other }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn fix_size() {
    let size = Size::new(100., 100.);
    let sized_box = SizedBox::from_size(size).have(Text("".to_string()).box_it());
    let (rect, child) = widget_and_its_children_box_rect(sized_box.box_it(), Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![Rect::from_size(size)]);
  }

  #[test]
  fn shrink_size() {
    let shrink = SizedBox::shrink().have(Text("".to_string()).box_it());
    let (rect, child) = widget_and_its_children_box_rect(shrink.box_it(), Size::new(500., 500.));

    assert_eq!(rect.size, Size::zero());
    assert_eq!(child, vec![Rect::zero()]);
  }

  #[test]
  fn expanded_size() {
    let wnd_size = Size::new(500., 500.);
    let expand_box = SizedBox::expanded()
      .have(Text("".to_string()).box_it())
      .box_it();
    let (rect, child) = widget_and_its_children_box_rect(expand_box, Size::new(500., 500.));

    assert_eq!(rect.size, wnd_size);
    assert_eq!(child, vec![Rect::from_size(wnd_size)]);
  }

  #[test]
  fn empty_box() {
    let size = Size::new(10., 10.);
    let empty_box = SizedBox::from_size(size);
    let (rect, child) = widget_and_its_children_box_rect(empty_box.box_it(), Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![]);
  }
}
