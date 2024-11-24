use crate::{events::focus_mgr::FocusHandle, prelude::*};

#[derive(Default)]
pub struct RequestFocus {
  handle: Option<FocusHandle>,
}

impl Declare for RequestFocus {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for RequestFocus {
  type Child = Widget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    fn_widget! {
      let mut child = FatObj::new(child);
      @ $child {
        on_mounted: move |e| {
          let track_id = $child.track_id();
          let handle = e.window().focus_mgr.borrow().focus_handle(track_id);
          $this.silent().handle = Some(handle);
        }
      }
      .into_widget()
      .try_unwrap_state_and_attach(this)
    }
    .into_widget()
  }
}
impl RequestFocus {
  pub fn request_focus(&self) {
    if let Some(h) = self.handle.as_ref() {
      h.request_focus();
    }
  }

  pub fn unfocus(&self) {
    if let Some(h) = self.handle.as_ref() {
      h.unfocus();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn dynamic_focus_node() {
    reset_test_env!();

    let widget = fn_widget! {
      let m = @MockBox {
        tab_index: 0i16,
        size: Size::default(),
      };
      let m = @ $m { tab_index: 0i16, };
      @ $m { tab_index: 0i16 }
    };

    let wnd = TestWindow::new(widget);
    let tree = wnd.tree();
    let id = tree.content_root();

    let mut cnt = 0;
    id.query_all_iter::<MixBuiltin>(tree)
      .for_each(|b| {
        if b.contain_flag(MixFlags::Focus) {
          cnt += 1;
        }
      });
    assert_eq!(cnt, 1);
  }
}
