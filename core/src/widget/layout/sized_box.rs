use crate::prelude::*;
pub use smallvec::{smallvec, SmallVec};

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[stateful]
#[derive(Widget)]
pub struct SizedBox {
  #[state]
  pub size: Size,
  pub child: Option<Box<dyn Widget>>,
}

impl SizedBox {
  /// Creates a box with the specified size.
  pub fn from_size<W: Widget>(size: Size, child: W) -> Self {
    Self { size, child: Some(child.box_it()) }
  }

  /// Creates a box that will become as large as its parent allows.
  pub fn expanded<W: Widget>(child: W) -> Self {
    Self {
      size: Size::new(f32::INFINITY, f32::INFINITY),
      child: Some(child.box_it()),
    }
  }

  /// Creates a box that will become as small as its parent allows.
  pub fn shrink<W: Widget>(child: W) -> Self {
    Self {
      size: Size::zero(),
      child: Some(child.box_it()),
    }
  }

  /// Creates a box with specified size without child.
  pub fn empty_box(size: Size) -> Self { Self { size, child: None } }
}

impl RenderWidget for SizedBox {
  type RO = SizedBoxState;
  #[inline]
  fn create_render_object(&self) -> Self::RO { SizedBoxState { size: self.size } }

  fn take_children(&mut self) -> Option<SmallVec<[Box<dyn Widget>; 1]>> {
    self.child.take().map(|w| smallvec![w])
  }
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
    let sized_box = SizedBox::from_size(size, Text("".to_string()));
    let (rect, child) = widget_and_its_children_box_rect(sized_box, Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![Rect::from_size(size)]);
  }

  #[test]
  fn shrink_size() {
    let shrink = SizedBox::shrink(Text("".to_string()));
    let (rect, child) = widget_and_its_children_box_rect(shrink, Size::new(500., 500.));

    assert_eq!(rect.size, Size::zero());
    assert_eq!(child, vec![Rect::zero()]);
  }

  #[test]
  fn expanded_size() {
    let wnd_size = Size::new(500., 500.);
    let expand_box = SizedBox::expanded(Text("".to_string()));
    let (rect, child) = widget_and_its_children_box_rect(expand_box, Size::new(500., 500.));

    assert_eq!(rect.size, wnd_size);
    assert_eq!(child, vec![Rect::from_size(wnd_size)]);
  }

  #[test]
  fn empty_box() {
    let size = Size::new(10., 10.);
    let empty_box = SizedBox::empty_box(size);
    let (rect, child) = widget_and_its_children_box_rect(empty_box, Size::new(500., 500.));
    assert_eq!(rect.size, size);
    assert_eq!(child, vec![]);
  }
}
