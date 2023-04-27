use crate::prelude::*;

bitflags::bitflags! {
  pub struct ClosePolicy: u8 {
    const NO_AUTO_CLOSE = 0x00;
    const CLOSE_ON_ESCAPE = 0x01;
    const CLOSE_ON_PRESS_OUTSIDE = 0x02;
  }
}

impl Default for ClosePolicy {
  fn default() -> ClosePolicy { ClosePolicy::CLOSE_ON_ESCAPE | ClosePolicy::CLOSE_ON_PRESS_OUTSIDE }
}

#[derive(Default)]
pub struct ModalConfig {
  pub close_on: ClosePolicy,
  pub dim_background: Option<Brush>,
}

impl ModalConfig {
  pub fn with_background<T: Into<Brush>>(brush: T) -> Self {
    Self {
      dim_background: Some(brush.into()),
      ..<_>::default()
    }
  }
}

#[derive(Declare)]
pub struct Overlay<W> {
  content: W,
  #[declare(default=Some(ModalConfig::default()), convert=strip_option)]
  modal: Option<ModalConfig>,
  #[declare(skip)]
  close_flag: Option<Stateful<bool>>,
}

impl<'a, W> StateRef<'a, Overlay<W>> {
  pub fn show<M>(&mut self, overlays: &OverlayMgr)
  where
    M: ImplMarker + 'static,
    W: IntoWidget<M> + 'static + Clone,
  {
    overlays.push_overlay(self.build())
  }

  pub fn close(&mut self) {
    if let Some(close_flag) = self.close_flag.take() {
      *close_flag.state_ref() = true;
    }
  }

  fn build<M>(&mut self) -> Widget
  where
    M: ImplMarker + 'static,
    W: IntoWidget<M> + 'static + Clone,
  {
    self.close();
    let close_flag: Stateful<bool> = Stateful::new(false);
    self.silent().close_flag = Some(close_flag.clone());
    let this = self.clone_stateful();
    widget! {
      states { close_flag: close_flag.clone() }
      DynWidget {
        dyns: (!*close_flag).then(
          || this.state_ref().inner_widget()
        ),
        stop_refresh: *close_flag,
      }
    }
    .into_widget()
  }

  fn inner_widget<M>(&self) -> Widget
  where
    M: ImplMarker + 'static,
    W: IntoWidget<M> + 'static + Clone,
  {
    let mut w = self.content.clone().into_widget();
    let Some(modal) = &self.modal else { return w };
    w = widget! {
      Container {
        size: Size::new(f32::INFINITY, f32::INFINITY),
        Widget::new(w)
      }
    }
    .into_widget();

    if modal.close_on.contains(ClosePolicy::CLOSE_ON_ESCAPE) {
      let this = self.clone_stateful();
      w = widget! {
        KeyDownListener {
          on_key_down: move |key| {
            if key.key == VirtualKeyCode::Escape {
              this.state_ref().close();
            }
          },
          widget::from(w)
        }
      }
      .into_widget();
    }

    if modal.close_on.contains(ClosePolicy::CLOSE_ON_PRESS_OUTSIDE) {
      let this = self.clone_stateful();
      w = widget! {
        TapListener {
          on_tap: move |ctx| {
            if ctx.current_target() == ctx.target() {
              this.state_ref().close();
            }
          },
          widget::from(w)
        }
      }
      .into_widget();
    }

    if let Some(dim) = modal.dim_background.clone() {
      w = widget! {
        BoxDecoration {
          background: dim,
          widget::from(w)
        }
      }
      .into_widget();
    }

    w
  }
}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::events::test_util::tap_on;

  use super::*;
  #[test]
  fn modal_overlay() {
    let overlay_id = Rc::new(RefCell::new(None));
    let tap_cnt = Rc::new(RefCell::new(0));

    let overlay_id2 = overlay_id.clone();
    let tap_cnt2 = tap_cnt.clone();
    let w = widget! {
      init { let mounted = overlay_id.clone(); let disposed = overlay_id.clone(); }
      Container {
        size: Size::new(50., 50.,),
        on_tap: move |ctx| {
          *tap_cnt.borrow_mut() += 1;
          overlay.show(ctx.overlays_mgr());
        }
      }
      Overlay {
        id: overlay,
        content: widget! {
          Container {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            size: Size::new(50., 50.,),
            on_mounted: move |ctx| *mounted.borrow_mut() = Some(ctx.id()),
            on_disposed: move |_| *disposed.borrow_mut() = None,
          }
        }
      }
    };

    let mut wnd = Window::default_mock(w, Some(Size::new(200., 200.)));
    wnd.draw_frame();

    let root = wnd.widget_tree.root();
    assert_eq!(root.children(&wnd.widget_tree.arena).count(), 1);

    tap_on(&mut wnd, 25., 25.);
    wnd.draw_frame();
    assert_eq!(root.children(&wnd.widget_tree.arena).count(), 2);
    assert_eq!(wnd.widget_tree.count(overlay_id2.borrow().unwrap()), 3);
    assert_eq!(*tap_cnt2.borrow(), 1);

    tap_on(&mut wnd, 25., 25.);
    wnd.draw_frame();
    assert!(overlay_id2.borrow().is_none());
    assert_eq!(*tap_cnt2.borrow(), 1);
    assert_eq!(root.children(&wnd.widget_tree.arena).count(), 1);
  }

  #[test]
  fn transparent_overlay() {
    let overlay_id = Rc::new(RefCell::new(None));
    let tap_cnt = Rc::new(RefCell::new(0));

    let overlay_id2 = overlay_id.clone();
    let tap_cnt2 = tap_cnt.clone();
    let w = widget! {
      init { let mounted = overlay_id.clone(); let disposed = overlay_id.clone(); }
      Container {
        size: Size::new(50., 50.,),
        on_tap: move |ctx| {
          *tap_cnt.borrow_mut() += 1;
          overlay.show(ctx.overlays_mgr());
        }
      }
      Overlay {
        id: overlay,
        modal: None,
        content: widget! {
          Container {
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            size: Size::new(50., 50.,),
            on_mounted: move |ctx| *mounted.borrow_mut() = Some(ctx.id()),
            on_disposed: move |_| *disposed.borrow_mut() = None,
          }
        }
      }
    };

    let mut wnd = Window::default_mock(w, Some(Size::new(200., 200.)));
    wnd.draw_frame();

    let root = wnd.widget_tree.root();
    assert!(root.children(&wnd.widget_tree.arena).count() == 1);

    tap_on(&mut wnd, 25., 25.);
    wnd.draw_frame();

    assert_eq!(wnd.widget_tree.count(overlay_id2.borrow().unwrap()), 3);
    assert_eq!(*tap_cnt2.borrow(), 1);
    assert_eq!(root.children(&wnd.widget_tree.arena).count(), 2);

    let old_id = (*overlay_id2.borrow()).unwrap();
    tap_on(&mut wnd, 25., 25.);
    wnd.draw_frame();
    assert_ne!((*overlay_id2.borrow()).unwrap(), old_id);
    assert_eq!(*tap_cnt2.borrow(), 2);
    assert_eq!(root.children(&wnd.widget_tree.arena).count(), 2);
  }
}
