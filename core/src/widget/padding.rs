use crate::prelude::*;

/// A widget that insets its child by the given padding.
#[derive(SingleChildWidget, Clone, Declare)]
pub struct Padding {
  #[declare(builtin)]
  pub padding: EdgeInsets,
}

impl RenderWidget for Padding {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let child = match ctx.single_render_child() {
      Some(c) => c,
      None => return Size::zero(),
    };

    let thickness = self.padding.thickness();
    let zero = Size::zero();
    let min = (clamp.min - thickness).max(zero);
    let max = (clamp.max - thickness).max(zero);
    // Shrink the clamp of child.
    let child_clamp = BoxClamp { min, max };

    let size = ctx.perform_render_child_layout(child, child_clamp);

    // Expand the size, so the child have padding.
    let size = clamp.clamp(size + thickness);
    ctx.update_size(child, size);

    // Update child's children position, let they have a correct position after
    // expanded with padding. padding.
    let (ctx, grandson_iter) = ctx.split_render_children_by(child);
    grandson_iter.for_each(|c| {
      let pos = ctx
        .widget_box_rect(c)
        .expect("The grandson render widget must performed layout")
        .origin
        + Vector::new(self.padding.left, self.padding.top);
      ctx.update_position(c, pos);
    });

    size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Padding {
  #[inline]
  pub fn new(padding: EdgeInsets) -> Self { Self { padding } }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn smoke() {
    let widget = Padding { padding: EdgeInsets::only_left(1.) }
      .have_child(
        Row::default()
          .have_child(SizedBox { size: Size::new(100., 100.) }.box_it())
          .box_it(),
      )
      .box_it();

    let mut wnd = Window::without_render(widget, Size::new(200., 200.));
    wnd.render_ready();
    let ctx = wnd.context();
    let padding_widget = ctx.widget_tree.root();

    assert_eq!(
      ctx.layout_store.layout_box_rect(padding_widget).unwrap(),
      Rect::from_size(Size::new(101., 100.))
    );

    let tree = &ctx.widget_tree;
    let row_widget = padding_widget.single_child(tree).unwrap();
    assert_eq!(
      ctx.layout_store.layout_box_rect(row_widget).unwrap(),
      Rect::from_size(Size::new(101., 100.))
    );

    let child_box = row_widget.single_child(tree).unwrap();
    assert_eq!(
      ctx.layout_store.layout_box_rect(child_box).unwrap(),
      Rect::new(Point::new(1., 0.), Size::new(100., 100.))
    );
  }
}
