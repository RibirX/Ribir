use ribir_core::prelude::*;

/// OnlySizedByParent implies that the parent is the only input into determining
/// the widget's size, so layout changes to the subtree do not trigger a parent
/// relayout.
#[derive(SingleChild, Query, Declare)]
pub struct OnlySizedByParent {}

impl Render for OnlySizedByParent {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if let Some(mut l) = ctx.first_child_layouter() {
      l.perform_widget_layout(clamp)
    } else {
      ZERO_SIZE
    }
  }

  fn only_sized_by_parent(&self) -> bool { true }

  fn paint(&self, _: &mut PaintingCtx) {
    // nothing to paint.
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{
    prelude::*,
    reset_test_env,
    test_helper::{split_value, MockBox, MockMulti, TestWindow},
  };

  use crate::layout::OnlySizedByParent;

  #[test]
  fn ignore_layout_changed() {
    reset_test_env!();
    let (parent_layout, parent_layout_writer) = split_value(0);
    let (child1_layout, child1_layout_writer) = split_value(0);
    let (child2_layout, child2_layout_writer) = split_value(0);
    let (child1_size, child1_size_writer) = split_value(Size::new(100., 100.));
    let (child2_size, child2_size_writer) = split_value(Size::new(100., 100.));
    let w = fn_widget! {
      let child1 = @MockBox {
        size: pipe!(*$child1_size),
        on_performed_layout: move |_| {
          *$child1_layout_writer.write() += 1;
        },
      };
      let child2 = @OnlySizedByParent {
        @MockBox {
          size: pipe!(*$child2_size),
          on_performed_layout: move |_| {
            *$child2_layout_writer.write() += 1;
          },
        }
      };
      @MockMulti {
        on_performed_layout: move |_| {
          *$parent_layout_writer.write() += 1;
        },
        @ { child1 }
        @ { child2 }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));

    // layout init from top down.
    wnd.draw_frame();
    assert_eq!(*parent_layout.read(), 1);
    assert_eq!(*child1_layout.read(), 1);
    assert_eq!(*child2_layout.read(), 1);

    // layout trigger from child without IgnoreLayoutChanged.
    {
      child1_size_writer.write().width = 200.;
    }
    wnd.draw_frame();
    assert_eq!(*parent_layout.read(), 2);
    assert_eq!(*child1_layout.read(), 2);
    assert_eq!(*child2_layout.read(), 1);

    // layout trigger from child wrap with IgnoreLayoutChanged.
    {
      child2_size_writer.write().width = 200.;
    }
    wnd.draw_frame();
    assert_eq!(*parent_layout.read(), 2);
    assert_eq!(*child1_layout.read(), 2);
    assert_eq!(*child2_layout.read(), 2);
  }
}
