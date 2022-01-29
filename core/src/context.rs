use std::{
  cell::Cell,
  collections::{HashMap, HashSet},
  pin::Pin,
};

use crate::prelude::{
  widget_tree::{WidgetNode, WidgetTree},
  AsAttrs, BoxedWidget, BoxedWidgetInner, Event, EventCommon, Key, StateAttr, WidgetId,
};
use ahash::RandomState;
use painter::{PaintCommand, Painter};
mod painting_context;
pub use painting_context::PaintingCtx;
mod event_context;
pub use event_context::EventCtx;
mod widget_context;
use ::text::shaper::TextShaper;
pub use widget_context::*;
use winit::{event::ModifiersState, window::CursorIcon};
mod layout_context;
pub(crate) mod layout_store;
pub use layout_context::*;
pub use layout_store::BoxClamp;
pub(crate) use layout_store::LayoutStore;
mod build_context;
pub use build_context::BuildCtx;

pub(crate) struct Context {
  pub layout_store: LayoutStore,
  pub widget_tree: Pin<Box<WidgetTree>>,
  pub painter: Painter,
  pub modifiers: ModifiersState,
  pub cursor: Cell<Option<CursorIcon>>,
  pub shaper: TextShaper,
  /// Store combination widgets changed.
  need_builds: HashSet<WidgetId, ahash::RandomState>,
}

impl Context {
  pub(crate) fn new(root: BoxedWidget, device_scale: f32) -> Self {
    let mut ctx = match root.0 {
      BoxedWidgetInner::Combination(c) => {
        let widget_tree = WidgetTree::new(WidgetNode::Combination(c));
        let mut ctx = Context::from_tree(widget_tree, device_scale);
        let tree = &ctx.widget_tree;
        let child = match tree.root().assert_get(tree) {
          WidgetNode::Combination(c) => c.build(&mut BuildCtx::new(&ctx, None, &**c)),
          WidgetNode::Render(_) => unreachable!(),
        };
        let root = tree.root();
        ctx.inflate_append(child, root);
        ctx
      }
      BoxedWidgetInner::Render(r) => {
        let widget_tree = WidgetTree::new(WidgetNode::Render(r));
        Context::from_tree(widget_tree, device_scale)
      }
      BoxedWidgetInner::SingleChild(s) => {
        let (rw, child) = s.unzip();
        let widget_tree = WidgetTree::new(WidgetNode::Render(rw));
        let mut ctx = Context::from_tree(widget_tree, device_scale);
        ctx.inflate_append(child, ctx.widget_tree.root());
        ctx
      }
      BoxedWidgetInner::MultiChild(m) => {
        let (rw, children) = m.unzip();
        let widget_tree = WidgetTree::new(WidgetNode::Render(rw));
        let mut ctx = Context::from_tree(widget_tree, device_scale);
        let root = ctx.widget_tree.root();
        children
          .into_iter()
          .for_each(|w| ctx.inflate_append(w, root));
        ctx
      }
    };
    ctx.mark_layout_from_root();
    ctx.shaper.font_db_mut().load_system_fonts();
    ctx
  }

  pub(crate) fn inflate_append(&mut self, widget: BoxedWidget, parent: WidgetId) {
    let mut stack = vec![(widget, parent)];

    while let Some((widget, p_wid)) = stack.pop() {
      match widget.0 {
        BoxedWidgetInner::Combination(c) => {
          let mut ctx = BuildCtx::new(self, Some(p_wid), &*c);
          let child = c.build(&mut ctx);
          let wid = p_wid.append_widget(WidgetNode::Combination(c), self.widget_tree.as_mut());

          stack.push((child, wid));
        }
        BoxedWidgetInner::Render(rw) => {
          p_wid.append_widget(WidgetNode::Render(rw), self.widget_tree.as_mut());
        }
        BoxedWidgetInner::SingleChild(s) => {
          let (rw, child) = s.unzip();
          let wid = p_wid.append_widget(WidgetNode::Render(rw), self.widget_tree.as_mut());
          stack.push((child, wid));
        }
        BoxedWidgetInner::MultiChild(m) => {
          let (rw, children) = m.unzip();
          let wid = p_wid.append_widget(WidgetNode::Render(rw), self.widget_tree.as_mut());
          children
            .into_iter()
            .rev()
            .for_each(|w| stack.push((w, wid)));
        }
      };
    }
  }

  pub(crate) fn bubble_event<D, E, F, Attr>(
    &self,
    widget: WidgetId,
    default: F,
    mut dispatch: D,
  ) -> E
  where
    F: FnOnce(&Context, WidgetId) -> E,
    D: FnMut(&Attr, &mut E),
    E: Event + AsMut<EventCommon>,
    Attr: 'static,
  {
    let mut event = default(self, widget);
    for wid in widget.ancestors(&self.widget_tree) {
      if let Some(attr) = wid.assert_get(&self.widget_tree).find_attr::<Attr>() {
        event.as_mut().current_target = wid;
        dispatch(attr, &mut event);
        if event.bubbling_canceled() {
          break;
        }
      }
    }

    event
  }

  pub(crate) fn draw_tree(&mut self) -> Vec<PaintCommand> {
    let tree = &self.widget_tree;
    let mut wid = Some(tree.root());

    while let Some(mut id) = wid {
      if let Some(rect) = self.layout_store.layout_box_rect(id) {
        self.painter.save();
        self.painter.translate(rect.min_x(), rect.min_y());
      }
      let n = id.assert_get(&tree);
      if let WidgetNode::Render(ref r) = n {
        r.paint(&mut PaintingCtx {
          id,
          tree,
          layout_store: &self.layout_store,
          painter: &mut self.painter,
        })
      }

      // try to access child
      wid = id.first_child(tree);

      loop {
        if wid.is_some() {
          break;
        }
        // if child is none, try to access sibling
        if self.layout_store.layout_box_rect(id).is_some() {
          self.painter.restore()
        }
        wid = id.next_sibling(tree);

        // if there is no more sibling, parent subtree finished, try to access
        // parent sibling
        if wid.is_none() {
          match id.parent(tree) {
            Some(p) => id = p,
            None => break,
          }
        }
      }
    }

    self.painter.finish()
  }

  pub fn find_attr<A: 'static>(&self, widget: WidgetId) -> Option<&A> {
    widget.get(&self.widget_tree).and_then(AsAttrs::find_attr)
  }

  /// mark this id combination widget has changed
  fn mark_changed(&mut self, id: WidgetId) {
    let tree = &mut self.widget_tree;
    if let WidgetNode::Render(_) = id.assert_get(tree) {
      self.layout_store.mark_needs_layout(id, &self.widget_tree);
      // paint widget not effect widget size, it's detect by single child or
      // parent max limit.
    } else {
      self.need_builds.insert(id);
    }
  }

  pub fn mark_layout_from_root(&mut self) {
    let tree = &self.widget_tree;
    if let Some(root) = tree.root().render_widget(tree) {
      self.layout_store.mark_needs_layout(root, tree);
    }
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed.
  pub fn tree_repair(&mut self) -> bool {
    let mut changed = false;
    while let Some(need_build) = self.pop_need_build_widget() {
      changed = self.repair_subtree(need_build) || changed;
      let rid = need_build.render_widget(&self.widget_tree).unwrap();
      self.layout_store.mark_needs_layout(rid, &self.widget_tree)
    }
    changed
  }

  pub fn state_change_dispatch(&mut self) {
    while let Some((id, silent)) = self.widget_tree.pop_changed_widgets() {
      self.mark_changed(id);
      if !silent {
        let attr = id
          .assert_get_mut(&mut self.widget_tree)
          .find_attr_mut::<StateAttr>();

        if let Some(attr) = attr {
          attr.changed_notify()
        }
      }
    }
  }

  #[allow(dead_code)]
  pub fn is_dirty(&self) -> bool {
    self
      .need_builds
      .iter()
      .any(|id| !id.is_dropped(&self.widget_tree))
      || self.layout_store.is_dirty(&self.widget_tree)
  }

  pub fn descendants(&self) -> impl Iterator<Item = WidgetId> + '_ {
    self.widget_tree.root().descendants(&self.widget_tree)
  }

  pub(crate) fn from_tree(widget_tree: Pin<Box<WidgetTree>>, device_scale: f32) -> Self {
    Context {
      layout_store: <_>::default(),
      widget_tree,
      painter: Painter::new(device_scale),
      cursor: <_>::default(),
      modifiers: <_>::default(),
      shaper: <_>::default(),
      need_builds: <_>::default(),
    }
  }

  /// Return the topmost need rebuild
  fn pop_need_build_widget(&mut self) -> Option<WidgetId> {
    let id = loop {
      let id = *self.need_builds.iter().next()?;
      if id.is_dropped(&self.widget_tree) {
        self.need_builds.remove(&id);
        continue;
      }
      break id;
    };

    let topmost = id
      .ancestors(&self.widget_tree)
      .skip(1)
      .fold(id, |mut id, p| {
        if self.need_builds.contains(&p) && !p.is_dropped(&self.widget_tree) {
          id = p
        }
        id
      });

    self.need_builds.remove(&topmost);
    Some(topmost)
  }

  fn repair_subtree(&mut self, sub_tree: WidgetId) -> bool {
    let c = match sub_tree.assert_get(&self.widget_tree) {
      WidgetNode::Combination(c) => c,
      WidgetNode::Render(_) => unreachable!("rebuild widget must be combination widget."),
    };

    let child = c.build(sub_tree.parent(&self.widget_tree), self);
    let child_id = sub_tree.single_child(&self.widget_tree).unwrap();

    let mut changed = false;
    let mut stack = vec![(child, child_id)];
    while let Some((w, wid)) = stack.pop() {
      match w.0 {
        BoxedWidgetInner::Combination(c) => {
          self.need_builds.remove(&wid);
          let mut ctx = BuildCtx::new(self, sub_tree.parent(&self.widget_tree), &*c);
          let child = c.build(&mut ctx);
          let new_id = self.replace_widget(WidgetNode::Combination(c), wid);
          changed = new_id.is_some() || changed;
          match new_id {
            Some(new_id) => self.inflate_append(child, new_id),
            None => stack.push((child, wid)),
          }
        }
        BoxedWidgetInner::Render(r) => {
          changed = self.replace_widget(WidgetNode::Render(r), wid).is_some() || changed;
        }
        BoxedWidgetInner::SingleChild(s) => {
          let (r, child) = s.unzip();
          let new_id = self.replace_widget(WidgetNode::Render(r), wid);
          changed = new_id.is_some() || changed;
          match new_id {
            Some(new_id) => self.inflate_append(child, new_id),
            None => stack.push((child, wid)),
          }
        }
        BoxedWidgetInner::MultiChild(m) => {
          let (r, children) = m.unzip();
          let new_id = self.replace_widget(WidgetNode::Render(r), wid);
          changed = new_id.is_some() || changed;
          match new_id {
            Some(new_id) => children
              .into_iter()
              .for_each(|c| self.inflate_append(c, new_id)),
            None => {
              let mut key_children = self.detach_key_children(wid);
              children.into_iter().for_each(|c| {
                match c.0.get_key().and_then(|k| key_children.remove(&*k)) {
                  Some(c_id) => {
                    wid.attach(c_id, &mut self.widget_tree);
                    stack.push((c, c_id));
                  }
                  None => self.inflate_append(c, wid),
                }
              });
              key_children.into_iter().for_each(|(_, k)| self.drop(k));
            }
          }
        }
      }
    }
    changed
  }

  // Collect and detach the child has key, and drop the others.
  fn detach_key_children(&mut self, wid: WidgetId) -> HashMap<Key, WidgetId, RandomState> {
    let mut key_children = HashMap::default();
    let mut child = wid.first_child(&self.widget_tree);
    while let Some(id) = child {
      child = id.next_sibling(&self.widget_tree);

      if let Some(key) = id.assert_get(&self.widget_tree).get_key().cloned() {
        id.detach(&mut self.widget_tree);
        key_children.insert(key, id);
      } else {
        self.drop(id)
      }
    }
    key_children
  }

  fn replace_widget(&mut self, w: WidgetNode, id: WidgetId) -> Option<WidgetId> {
    let new_id = self.widget_tree.as_mut().replace_widget(w, id);
    if new_id.is_some() {
      self.drop(id)
    }
    new_id
  }

  fn drop(&mut self, id: WidgetId) {
    id.descendants(&self.widget_tree).for_each(|c| {
      self.layout_store.remove(c);
    });
    id.remove_subtree(&mut self.widget_tree)
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use test::Bencher;

  use super::*;
  use crate::{
    prelude::BoxWidget,
    test::{embed_post::EmbedPost, key_embed_post::EmbedPostWithKey, recursive_row::RecursiveRow},
  };

  fn test_sample_create(width: usize, depth: usize) -> Context {
    let root = RecursiveRow { width, depth };
    Context::new(root.box_it(), 1.)
  }

  #[test]
  fn drop_info_clear() {
    let post = EmbedPost::new(3);
    let mut ctx = Context::new(post.box_it(), 1.);
    assert_eq!(ctx.widget_tree.count(), 20);
    ctx.mark_changed(ctx.widget_tree.root());
    ctx.drop(ctx.widget_tree.root());

    assert_eq!(ctx.is_dirty(), false);
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost::new(1000);
      Context::new(post.box_it(), 1.);
    });
  }

  #[bench]
  fn inflate_50_pow_2(b: &mut Bencher) { b.iter(|| test_sample_create(50, 2)) }

  #[bench]
  fn inflate_100_pow_2(b: &mut Bencher) { b.iter(|| test_sample_create(100, 2)) }

  #[bench]
  fn inflate_10_pow_4(b: &mut Bencher) { b.iter(|| test_sample_create(10, 4)) }

  #[bench]
  fn inflate_10_pow_5(b: &mut Bencher) { b.iter(|| test_sample_create(10, 5)) }

  #[bench]
  fn repair_5_x_1000(b: &mut Bencher) {
    let post = EmbedPostWithKey::new(1000);
    let mut ctx = Context::new(post.box_it(), 1.);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair()
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let mut ctx = test_sample_create(50, 2);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let mut ctx = test_sample_create(100, 2);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let mut ctx = test_sample_create(10, 4);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let mut ctx = test_sample_create(10, 5);
    b.iter(|| {
      ctx.mark_changed(ctx.widget_tree.root());
      ctx.tree_repair();
    })
  }

  #[test]
  fn repair() {
    let mut ctx = test_sample_create(1, 1);
    ctx.mark_changed(ctx.widget_tree.root());
    assert!(!ctx.need_builds.is_empty());
    ctx.tree_repair();
    assert!(ctx.need_builds.is_empty());
  }
}
