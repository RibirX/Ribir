use crate::prelude::*;

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(Debug)]
pub struct SizedBox {
  pub size: Size,
  pub child: BoxWidget,
}

#[derive(Debug)]
pub struct SizedBoxRender {
  size: Size,
}

impl SizedBox {
  /// Creates a box with the specified size.
  pub fn from_size<W: Widget>(size: Size, child: W) -> Self {
    Self {
      size,
      child: child.box_it(),
    }
  }

  /// Creates a box that will become as large as its parent allows.
  pub fn expanded<W: Widget>(child: W) -> Self {
    Self {
      size: Size::new(f32::INFINITY, f32::INFINITY),
      child: child.box_it(),
    }
  }

  /// Creates a box that will become as small as its parent allows.
  pub fn shrink<W: Widget>(child: W) -> Self {
    Self {
      size: Size::zero(),
      child: child.box_it(),
    }
  }
}

impl RenderWidget for SizedBox {
  type RO = SizedBoxRender;
  #[inline]
  fn create_render_object(&self) -> Self::RO { SizedBoxRender { size: self.size } }
}

single_child_widget_base_impl!(SizedBox);

impl SingleChildWidget for SizedBox {
  fn take_child(&mut self) -> BoxWidget {
    let hold = PhantomWidget.box_it();
    std::mem::replace(&mut self.child, hold)
  }
}

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
    let mut child = child_iter
      .next()
      .expect("SizedBox must have a single child.");
    debug_assert!(child_iter.next().is_none());
    child.perform_layout(BoxClamp {
      min: size,
      max: size,
    });
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
      window::NoRenderWindow::without_render(sized_box.box_it(), DeviceSize::new(500, 400));
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

    let sized_box = SizedBox::from_size(size, Text("".to_string()));
    check(sized_box, size);

    let expand_box = SizedBox::expanded(Text("".to_string()));
    check(expand_box, Size::new(500., 500.));

    let shrink = SizedBox::shrink(Text("".to_string()));
    check(shrink, Size::zero());
  }
}
