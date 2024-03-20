use crate::{events::focus_mgr::FocusHandle, prelude::*};

#[derive(Query, Default)]
pub struct RequestFocus {
  handle: Option<FocusHandle>,
}

impl Declare for RequestFocus {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl ComposeChild for RequestFocus {
  type Child = Widget;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> impl WidgetBuilder {
    fn_widget! {
      @$child {
        on_mounted: move |e| {
          let handle = e.window().focus_mgr.borrow().focus_handle(e.id);
          $this.silent().handle = Some(handle);
        }
      }
      .build(ctx!())
      .attach_state_data(this, ctx!())
    }
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
      @MixBuiltin {
        tab_index: 0i16, auto_focus: false,
        @MixBuiltin {
          tab_index: 0i16, auto_focus: false,
          @MixBuiltin {
            tab_index: 0i16, auto_focus: false,
            @MockBox {
              size: Size::default(),
            }
          }
        }
      }
    };

    let wnd = TestWindow::new(widget);
    let tree = wnd.widget_tree.borrow();
    let id = tree.content_root();
    let node = id.get(&tree.arena).unwrap();
    let mut cnt = 0;
    node.query_type_inside_first(|b: &MixBuiltin| {
      if b.contain_flag(BuiltinFlags::Focus) {
        cnt += 1;
      }
      true
    });

    assert_eq!(cnt, 1);
  }
}
