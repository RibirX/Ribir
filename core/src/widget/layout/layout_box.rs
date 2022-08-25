use crate::prelude::*;

/// Widget let user to access the layout result of its child.
#[derive(Declare)]
pub struct LayoutBox {
  #[declare(skip)]
  /// the rect box of its child and the coordinate is relative to its parent.
  rect: Rect,
}

impl ComposeSingleChild for LayoutBox {
  fn compose_single_child(this: StateWidget<Self>, child: Widget, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      ExprWidget {
        expr: child,
        on_performed_layout: move |ctx| this.rect = ctx.box_rect().unwrap()
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{prelude::Window, test::root_and_children_rect};

  #[test]
  fn smoke() {
    let w = widget! {
      Row {
        LayoutBox {
          id: layout_box,
          SizedBox { size: Size::new(100., 200.) }
        }
        SizedBox { size: layout_box.rect.size }
      }
    };
    let mut wnd = Window::wgpu_headless(w, DeviceSize::new(500, 500));
    wnd.draw_frame();
    let (rect, _) = root_and_children_rect(&wnd);
    assert_eq!(rect.size, Size::new(200., 200.));
  }
}
