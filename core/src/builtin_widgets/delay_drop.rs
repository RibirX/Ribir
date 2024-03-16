use crate::prelude::*;

/// A widget that can delay drop its child until the `delay_drop_until` field be
/// set to `true`.
///
/// This widget not effect the widget lifecycle, if the widget is dispose but
/// the `delay_drop_until` is `false`, it's not part of the widget tree anymore
/// but not drop immediately, is disposed in `logic`, but not release resource.
/// It's be isolated from the widget tree and can layout and paint normally.
///
/// Once the `delay_drop_until` field be set to `true`, the widget will be
/// dropped.
///
/// It's useful when you need run a leave animation for a widget.
#[derive(Query, Default)]
pub struct DelayDrop {
  pub delay_drop_until: bool,
}

impl Declare for DelayDrop {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl ComposeChild for DelayDrop {
  type Child = Widget;
  #[inline]
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      let modifies = this.raw_modifies();
      child.attach_state_data(this, ctx!()).dirty_subscribe(modifies, ctx!())
    }
  }
}

#[cfg(test)]
mod tests {
  use std::cell::Ref;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn smoke() {
    reset_test_env!();

    let delay_drop = Stateful::new(false);
    let c_delay_drop = delay_drop.clone_writer();
    let remove_widget = Stateful::new(false);
    let c_remove_widget = remove_widget.clone_writer();
    let mut wnd = TestWindow::new(fn_widget! {
      pipe! {
        if *$remove_widget {
          Void.widget_build(ctx!())
        } else {
          FatObj::new(Void)
            .delay_drop_until(pipe!(*$delay_drop))
            .widget_build(ctx!())
        }
      }
    });

    fn tree_arena(wnd: &TestWindow) -> Ref<TreeArena> {
      let tree = wnd.widget_tree.borrow();
      Ref::map(tree, |t| &t.arena)
    }

    let root = wnd.widget_tree.borrow().content_root();
    wnd.draw_frame();

    *c_remove_widget.write() = true;
    wnd.draw_frame();
    assert!(!root.is_dropped(&tree_arena(&wnd)));

    *c_delay_drop.write() = true;
    wnd.draw_frame();
    assert!(root.is_dropped(&tree_arena(&wnd)));
  }
}
