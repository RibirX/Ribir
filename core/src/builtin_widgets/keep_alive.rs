use crate::prelude::*;

/// A widget will be painted event if it has been dispose until the `keep_alive`
/// field be set to `false` or its parent is dropped.
///
/// This widget not effect the widget lifecycle, if the widget is dispose but
/// the `keep_alive` is `true`, it's not part of the widget tree anymore
/// but not drop immediately, is disposed in `logic`, but not release resource.
/// It's be isolated from the widget tree and can layout and paint normally.
///
/// Once the `keep_alive` field be set to `false`, the widget will be
/// dropped.
///
/// It's useful when you need run a leave animation for a widget.
#[derive(Query, Default)]
pub struct KeepAlive {
  pub keep_alive: bool,
}

impl Declare for KeepAlive {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl ComposeChild for KeepAlive {
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

    let keep_alive = Stateful::new(true);
    let c_keep_alive = keep_alive.clone_writer();
    let remove_widget = Stateful::new(false);
    let c_remove_widget = remove_widget.clone_writer();
    let mut wnd = TestWindow::new(fn_widget! {
      pipe! {
        if *$remove_widget {
          Void.build(ctx!())
        } else {
          FatObj::new(Void)
            .keep_alive(pipe!(*$keep_alive))
            .build(ctx!())
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

    *c_keep_alive.write() = false;
    wnd.draw_frame();
    assert!(root.is_dropped(&tree_arena(&wnd)));
  }
}
