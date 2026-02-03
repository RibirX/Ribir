use ribir_core::prelude::*;

/// NoAffectedParentSize implies that the layout changes to the subtree do not
/// trigger a parent relayout.
#[derive(SingleChild, Declare)]
pub struct NoAffectedParentSize {}

// `NoAffectedParentSize` must be an independent node in the widget tree.
// Therefore, any modifications to its child should terminate at
// `NoAffectedParentSize`. Otherwise, if its host is dirty, it implies that the
// `NoAffectedParentSize` node is also dirty, and its parent must be marked as
// dirty. For instance, if `w2` in a Row[w1, NoAffectedParentSize<w2>] is
// dirty, the Row requires a relayout.
impl Render for NoAffectedParentSize {
  fn measure(&self, clamp: BoxClamp, ctx: &mut MeasureCtx) -> Size {
    ctx
      .perform_single_child_layout(clamp)
      .unwrap_or(ZERO_SIZE)
  }

  fn size_affected_by_child(&self) -> bool { false }

  fn hit_test(&self, _ctx: &mut HitTestCtx, _pos: Point) -> HitTest {
    // The hit test is always delegated to its child,
    // just return { hit: false, can_hit_child: true } here.
    HitTest { hit: false, can_hit_child: true }
  }

  #[cfg(feature = "debug")]
  fn debug_name(&self) -> std::borrow::Cow<'static, str> {
    std::borrow::Cow::Borrowed("noAffectedParentSize")
  }
}

#[cfg(test)]
mod tests {
  use ribir_core::{prelude::*, reset_test_env, test_helper::*};

  use crate::layout::NoAffectedParentSize;

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
        size: pipe!(*$read(child1_size)),
        on_performed_layout: move |_| {
          *$write(child1_layout_writer) += 1;
        },
      };
      let child2 = @NoAffectedParentSize {
        @MockBox {
          size: pipe!(*$read(child2_size)),
          on_performed_layout: move |_| {
            *$write(child2_layout_writer) += 1;
          },
        }
      };
      @MockMulti {
        on_performed_layout: move |_| {
          *$write(parent_layout_writer) += 1;
        },
        @ { child1 }
        @ { child2 }
      }
    };

    let wnd = TestWindow::new_with_size(w, Size::new(200., 200.));

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
