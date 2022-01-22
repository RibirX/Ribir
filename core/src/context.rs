use std::cmp::Reverse;

use crate::prelude::{
  render_tree::{RenderEdge, RenderTree},
  widget_tree::WidgetTree,
  BoxClamp, BoxedWidget, RenderId, WidgetId,
};
use painter::{Painter, Size};
mod painting_context;
pub(crate) use painting_context::draw_tree;
pub use painting_context::PaintingCtx;
mod event_context;
pub use event_context::EventCtx;
mod widget_context;
use ::text::shaper::TextShaper;
pub use widget_context::*;
use winit::{event::ModifiersState, window::CursorIcon};
pub(crate) mod layout_store;
mod render_context;
pub(crate) use layout_store::LayoutStore;
pub use render_context::*;

pub struct Context {
  pub(crate) render_tree: RenderTree,
  pub(crate) layout_store: LayoutStore,
  pub(crate) widget_tree: WidgetTree,
  pub(crate) painter: Painter,
  pub(crate) modifiers: ModifiersState,
  pub(crate) cursor: Option<CursorIcon>,
  pub(crate) shaper: TextShaper,
}

impl Context {
  pub fn new(root: BoxedWidget, device_scale: f32) -> Self {
    let mut render_tree = RenderTree::default();
    let mut widget_tree = WidgetTree::default();
    let mut layout_store = <_>::default();

    widget_tree.set_root(root, &mut render_tree, &mut layout_store);
    Context {
      render_tree,
      layout_store,
      widget_tree,
      painter: Painter::new(device_scale),
      cursor: None,
      modifiers: <_>::default(),
      shaper: <_>::default(),
    }
  }

  /// Do the work of computing the layout for all node which need, always layout
  /// from the root to leaf. Return if any node has really computing the layout.
  pub fn layout(&mut self, win_size: Size) -> bool {
    loop {
      if !self.layout_store.has_need_layout() {
        break false;
      }
      let needs_layout = self.layout_store.take_needs_layout();

      needs_layout.iter().for_each(|Reverse((_depth, rid))| {
        if self.layout_store.layout_clamp(*rid).is_none() {
          let clamp = BoxClamp { min: Size::zero(), max: win_size };
          RenderCtx::new(*rid, self).perform_layout(clamp);
        }
      });

      if !self.layout_store.has_need_layout() {
        break !needs_layout.is_empty();
      }
    }
  }

  /// Mark widget need layout, if give a None value, the root will be mark
  pub fn mark_needs_layout(&mut self, rid: Option<RenderId>) {
    let rid = rid.unwrap_or_else(|| self.render_tree.root().unwrap());
    self
      .layout_store
      .mark_needs_layout(rid, &mut self.render_tree)
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed. The render tree will also
  /// react widget tree's change.
  pub fn tree_repair(&mut self) -> bool {
    self
      .widget_tree
      .repair(&mut self.render_tree, &mut self.layout_store)
  }

  pub fn descendants(&self) -> impl Iterator<Item = WidgetId> + '_ {
    let wid = self.widget_tree.root().unwrap();
    wid.descendants(&self.widget_tree)
  }

  /// Split self as self and an iterator of ids of `id` and its descendants, in
  /// tree order.
  pub fn split_traverse(&mut self, id: RenderId) -> (&mut Self, impl Iterator<Item = RenderEdge>) {
    // Safety: context provide no way to modify tree struct.
    let r_tree = unsafe { &mut *(&mut self.render_tree as *mut RenderTree) };
    (self, id.traverse(r_tree))
  }

  fn split_r_tree(&mut self) -> (&mut Self, &mut RenderTree) {
    // Safety: split `RenderTree` as two mutable reference is safety, because it's a
    // private inner mutable and promise export only use to access inner object and
    // never modify the tree struct by this reference.
    let r_tree = unsafe { &mut *(&mut self.render_tree as *mut RenderTree) };
    (self, r_tree)
  }
}
