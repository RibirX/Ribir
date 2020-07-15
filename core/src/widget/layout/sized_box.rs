use crate::prelude::*;
pub use smallvec::{smallvec, SmallVec};

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(Debug)]
pub struct SizedBox {
  pub size: Size,
  pub child: Option<BoxWidget>,
}

#[derive(Debug)]
pub struct SizedBoxRender {
  size: Size,
}

impl SizedBox {
  /// Creates a box with the specified size.
  pub fn from_size<W: Widget>(size: Size, child: Option<W>) -> Self {
    Self {
      size,
      child: child.map(|w| w.box_it()),
    }
  }

  /// Creates a box that will become as large as its parent allows.
  pub fn expanded<W: Widget>(child: Option<W>) -> Self {
    Self {
      size: Size::new(f32::INFINITY, f32::INFINITY),
      child: child.map(|w| w.box_it()),
    }
  }

  /// Creates a box that will become as small as its parent allows.
  pub fn shrink<W: Widget>(child: Option<W>) -> Self {
    Self {
      size: Size::zero(),
      child: child.map(|w| w.box_it()),
    }
  }
}

impl RenderWidget for SizedBox {
  type RO = SizedBoxRender;
  #[inline]
  fn create_render_object(&self) -> Self::RO { SizedBoxRender { size: self.size } }

  fn take_children(&mut self) -> Option<SmallVec<[BoxWidget; 1]>> {
    self.child.take().map(|w| smallvec![w])
  }
}

render_widget_base_impl!(SizedBox);

impl RenderObject for SizedBoxRender {
  type Owner = SizedBox;

  fn update(&mut self, owner_widget: &Self::Owner, ctx: &mut UpdateCtx) {
    if self.size != owner_widget.size {
      self.size = owner_widget.size;
      ctx.mark_needs_layout();
    }
  }

  fn perform_layout(&mut self, clamp: BoxClamp, ctx: &mut RenderCtx) -> Size {
    let size = clamp.clamp(self.size);
    let mut child_iter = ctx.children();
    let child = child_iter.next();
    debug_assert!(child_iter.next().is_none());
    if let Some(mut child_ctx) = child {
      child_ctx.perform_layout(BoxClamp {
        min: size,
        max: size,
      });
    }
    size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint<'a>(&'a self, _: &mut PaintingContext<'a>) {
    // nothing to paint, just a layout widget.
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn check(sized_box: SizedBox, check_size: Size) {
    let mut window =
      window::NoRenderWindow::without_render(sized_box.box_it(), Size::new(500., 400.));
    window.render_ready();

    let r_tree = window.render_tree();
    let info = r_tree.layout_info();
    assert_eq!(info.len(), 2);
    let mut iter = info.values();
    assert_eq!(iter.next().unwrap().rect.unwrap().size, check_size);
    assert_eq!(iter.next().unwrap().rect.unwrap().size, check_size);
  }

  #[test]
  fn smoke() {
    let size = Size::new(100., 100.);

    let sized_box = SizedBox::from_size(size, Some(Text("".to_string())));
    check(sized_box, size);

    let expand_box = SizedBox::expanded(Some(Text("".to_string())));
    check(expand_box, Size::new(500., 400.));

    let shrink = SizedBox::shrink(Some(Text("".to_string())));
    check(shrink, Size::zero());
  }
}
