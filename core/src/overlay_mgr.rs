use std::{
  cell::RefCell,
  mem::{swap, take},
};

use crate::{impl_query_self_only, prelude::*};

#[derive(Clone)]
pub struct OverlayMgr {
  new_overlays: Stateful<OverlayMgrInner>,
}

#[derive(Default)]
struct OverlayMgrInner(RefCell<Vec<Widget>>);

impl OverlayMgrInner {
  fn push(&mut self, w: Widget) { self.0.borrow_mut().push(w) }
  fn drain(&self) -> Vec<Widget> { take(&mut *self.0.borrow_mut()) }
}

impl Default for OverlayMgr {
  fn default() -> Self {
    Self {
      new_overlays: Stateful::new(OverlayMgrInner::default()),
    }
  }
}

impl OverlayMgr {
  pub fn push_overlay(&self, w: Widget) { self.new_overlays.state_ref().push(w) }

  pub(crate) fn bind_widget(&self, w: Widget) -> Widget {
    widget! {
      states { overlays: self.new_overlays.clone() }
      OverlaysRoot {
        new_overlays: RefCell::new(overlays.drain()),
        widget::from(w)
      }
    }
    .into_widget()
  }
}

#[derive(MultiChild, Declare)]
pub(crate) struct OverlaysRoot {
  new_overlays: RefCell<Vec<Widget>>,
}

impl Render for OverlaysRoot {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if self.new_overlays.borrow().len() > 0 {
      let mut widgets = vec![];
      swap(&mut widgets, &mut *self.new_overlays.borrow_mut());
      let id = ctx.id();
      widgets.into_iter().for_each(|w| {
        let wid = w.into_subtree(None, ctx.arena, ctx.wnd_ctx).unwrap();
        id.append(wid, ctx.arena);
        wid.on_mounted_subtree(ctx.arena, ctx.store, ctx.wnd_ctx, ctx.dirty_set);
      });
    }
    let w = Stack { fit: StackFit::Passthrough };
    w.perform_layout(clamp, ctx)
  }

  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for OverlaysRoot {
  impl_query_self_only!();
}
