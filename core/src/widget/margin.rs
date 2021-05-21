use crate::prelude::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgeInsets {
  pub left: f32,
  pub right: f32,
  pub bottom: f32,
  pub top: f32,
}

/// A widget that crate space around its child.
#[derive(Widget)]
pub struct Margin {
  pub margin: EdgeInsets,
  pub child: Box<dyn Widget>,
}

#[derive(Debug)]
pub struct MarginRender(EdgeInsets);

impl RenderWidget for Margin {
  type RO = MarginRender;
  #[inline]
  fn create_render_object(&self) -> Self::RO { MarginRender(self.margin.clone()) }

  fn take_children(&mut self) -> Option<SmallVec<[Box<dyn Widget>; 1]>> {
    Some(smallvec![std::mem::replace(
      &mut self.child,
      PhantomWidget.box_it()
    )])
  }
}

impl RenderObject for MarginRender {
  type Owner = Margin;
  fn update(&mut self, owner_widget: &Self::Owner, ctx: &mut UpdateCtx) {
    if owner_widget.margin != self.0 {
      ctx.mark_needs_layout();
    }
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let thickness = self.0.thickness();
    let zero = Size::zero();
    let min = (clamp.min - thickness).max(zero);
    let max = (clamp.max - thickness).max(zero);
    let child_clamp = BoxClamp { min, max };

    debug_assert_eq!(ctx.children().count(), 1);
    let mut child = ctx.children().next().expect("Margin must have one child");
    let size = child.perform_layout(child_clamp);
    child.update_position(Point::new(self.0.left, self.0.top));

    clamp.clamp(size + thickness)
  }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {}
}

impl EdgeInsets {
  #[inline]
  pub fn all(value: f32) -> Self {
    Self {
      top: value,
      left: value,
      bottom: value,
      right: value,
    }
  }

  #[inline]
  pub fn only_left(left: f32) -> Self { Self { left, ..Default::default() } }

  #[inline]
  pub fn only_right(right: f32) -> Self { Self { right, ..Default::default() } }
  #[inline]
  pub fn only_bottom(bottom: f32) -> Self { Self { bottom, ..Default::default() } }

  #[inline]
  pub fn only_top(top: f32) -> Self { Self { top, ..Default::default() } }

  #[inline]
  pub fn symmetrical(vertical: f32, horizontal: f32) -> Self {
    Self {
      top: vertical,
      bottom: vertical,
      left: horizontal,
      right: horizontal,
    }
  }

  #[inline]
  pub fn vertical(vertical: f32) -> Self {
    Self {
      top: vertical,
      bottom: vertical,
      ..Default::default()
    }
  }

  #[inline]
  pub fn horizontal(horizontal: f32) -> Self {
    Self {
      left: horizontal,
      right: horizontal,
      ..Default::default()
    }
  }

  pub fn thickness(&self) -> Size { Size::new(self.right + self.left, self.bottom + self.top) }

  /// Convert to an array by the top, right, bottom, left order.
  #[inline]
  pub fn to_array(&self) -> [f32; 4] { [self.top, self.right, self.bottom, self.left] }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn smoke() {
    let widget =
      SizedBox::empty_box(Size::new(100., 100.)).with_margin(EdgeInsets::symmetrical(1., 1.));
    let (rect, children) = widget_and_its_children_box_rect(widget, Size::new(200., 200.));

    assert_eq!(rect, Rect::from_size(Size::new(102., 102.)));
    assert_eq!(
      &children,
      &[Rect::new(Point::new(1., 1.0), Size::new(100., 100.))]
    );
  }
}
