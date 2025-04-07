use std::cmp::Ordering;

use indextree::{Arena, NodeId};

use crate::{
  prelude::*,
  window::{DelayEvent, WindowId},
};

#[derive(Debug)]
pub(crate) struct FocusManager {
  /// store current focusing node, and its position in tab_orders.
  focusing: Option<WidgetId>,
  request_focusing: Option<Option<WidgetId>>,
  frame_auto_focus: Vec<WidgetId>,
  focus_widgets: Vec<WidgetId>,
  node_ids: ahash::HashMap<WidgetId, NodeId>,
  arena: Arena<FocusNodeInfo>,
  root: NodeId,
  wnd_id: WindowId,
}

pub struct FocusHandle {
  wid: TrackId,
  wnd_id: WindowId,
}

impl FocusHandle {
  pub(crate) fn request_focus(&self) {
    if let Some(wnd) = AppCtx::get_window(self.wnd_id) {
      let wid = self.wid.get();
      wnd.focus_mgr.borrow_mut().request_focus_to(wid);
    }
  }

  pub(crate) fn unfocus(&self) {
    if let Some(wnd) = AppCtx::get_window(self.wnd_id) {
      if wnd.focus_mgr.borrow().focusing == self.wid.get() {
        wnd.focus_mgr.borrow_mut().request_focus_to(None);
      }
    }
  }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub(crate) enum FocusType {
  Scope,
  Node,
}

#[derive(Debug)]
pub(crate) struct FocusNodeInfo {
  pub scope_cnt: u32,
  pub node_cnt: u32,
  pub wid: Option<WidgetId>,
}

impl FocusNodeInfo {
  fn new(wid: WidgetId) -> Self { FocusNodeInfo { scope_cnt: 0, node_cnt: 0, wid: Some(wid) } }

  fn has_focus_node(&self) -> bool { self.node_cnt > 0 }

  fn has_focus_scope(&self) -> bool { self.scope_cnt > 0 }

  fn add_focus(&mut self, ft: FocusType) {
    match ft {
      FocusType::Node => self.node_cnt += 1,
      FocusType::Scope => self.scope_cnt += 1,
    }
  }

  fn remove_focus(&mut self, ft: FocusType) {
    match ft {
      FocusType::Node => self.node_cnt -= 1,
      FocusType::Scope => self.scope_cnt -= 1,
    }
  }

  fn is_empty(&self) -> bool { self.node_cnt + self.scope_cnt == 0 }
}

impl FocusManager {
  pub(crate) fn new(wnd_id: WindowId) -> Self {
    let mut arena = Arena::new();
    let root = arena.new_node(FocusNodeInfo { node_cnt: 0, scope_cnt: 1, wid: None });
    Self {
      wnd_id,
      focus_widgets: Vec::new(),
      frame_auto_focus: vec![],
      request_focusing: None,
      focusing: None,
      node_ids: ahash::HashMap::default(),
      arena,
      root,
    }
  }

  pub(crate) fn window(&self) -> Sc<Window> {
    AppCtx::get_window(self.wnd_id).expect("The window of `FocusManager` has already dropped.")
  }

  pub(crate) fn add_focus_node(&mut self, wid: WidgetId, auto_focus: bool, focus_type: FocusType) {
    if let Some(id) = self.node_ids.get(&wid) {
      let node = self.arena[*id].get_mut();
      node.add_focus(focus_type);
    } else {
      let mut node = FocusNodeInfo::new(wid);
      node.add_focus(focus_type);
      let node_id = self.arena.new_node(node);
      self.node_ids.insert(wid, node_id);

      let wnd = self.window();
      let tree = wnd.tree();
      let mut it = wid.ancestors(tree).skip(1);
      let parent = it
        .find_map(|id| self.node_ids.get(&id))
        .unwrap_or(&self.root);
      self.insert_node(*parent, node_id, wid, tree);
    }

    if auto_focus && focus_type == FocusType::Node {
      self.frame_auto_focus.push(wid);
    }
  }

  pub(crate) fn focus_handle(&self, wid: TrackId) -> FocusHandle {
    FocusHandle { wid, wnd_id: self.window().id() }
  }

  pub(crate) fn remove_focus_node(&mut self, wid: WidgetId, focus_type: FocusType) {
    let Some(id) = self.node_ids.get(&wid).cloned() else {
      return;
    };

    let node = self.arena[id].get_mut();
    node.remove_focus(focus_type);
    if node.is_empty() {
      id.remove(&mut self.arena);
      self.node_ids.remove(&wid);
    }
  }

  pub fn next_focus(&mut self, arena: &WidgetTree) -> Option<WidgetId> {
    let request_focus = self.request_focusing.take();
    let autos = self.frame_auto_focus.drain(..);
    let next_focus = request_focus
      .into_iter()
      .chain(autos.map(Some))
      .find(|request| {
        request
          .as_ref()
          .is_none_or(|id| !id.is_dropped(arena))
      });

    let focusing = next_focus
      .unwrap_or(self.focusing)
      .filter(|node_id| self.ignore_scope_id(*node_id).is_none());
    let focus_node = focusing.and_then(|wid| self.node_ids.get(&wid));
    let info = focus_node.and_then(|id: &NodeId| self.get(*id));

    let focus_to = if let Some(node) = info {
      if node.has_focus_scope() {
        let scope = self.scope_property(node.wid);
        if node.has_focus_node() && !scope.skip_host {
          node.wid
        } else if !scope.skip_descendants {
          self
            .focus_step_in_scope(*focus_node.unwrap(), None, false)
            .and_then(|id| self.assert_get(id).wid)
        } else {
          None
        }
      } else {
        node.wid
      }
    } else {
      None
    };
    focus_to
  }

  fn focus_move_circle(&mut self, backward: bool) {
    let has_focus = self.focusing.is_some();
    let mut wid = self.focus_step(self.focusing, backward);
    if wid.is_none() && has_focus {
      wid = self.focus_step(wid, backward);
    }
    self.request_focus_to(wid);
  }

  fn focus_step(&mut self, focusing: Option<WidgetId>, backward: bool) -> Option<WidgetId> {
    let mut node_id = focusing
      .and_then(|id| self.node_ids.get(&id))
      .copied();
    let mut scope_id = node_id
      .and_then(|id| self.scope_id(id))
      .or(Some(self.root));
    loop {
      scope_id?;
      let next = self.focus_step_in_scope(scope_id.unwrap(), node_id, backward);
      if let Some(id) = next {
        return self.get(id).and_then(|n| n.wid);
      } else {
        node_id = scope_id;
        scope_id = self.scope_id(node_id.unwrap());
      }
    }
  }

  fn collect_tab_index_in_scope(
    &self, scope_id: NodeId, backward: bool,
  ) -> Vec<(i16, NodeId, FocusType)> {
    let scope_tab_type = |id, has_focus_node: bool| {
      let mut v = vec![];
      let node = self.scope_property(id);

      if has_focus_node && !node.skip_host {
        v.push(FocusType::Node);
      }
      if !node.skip_descendants {
        v.push(FocusType::Scope);
      }

      v
    };
    let is_scope = |id| self.assert_get(id).has_focus_scope();
    let node_type = |id| {
      self
        .arena
        .get(id)
        .map(|n| n.get())
        .map_or(vec![], |node| {
          if node.has_focus_scope() {
            scope_tab_type(node.wid, node.has_focus_node())
          } else if node.has_focus_node() {
            vec![FocusType::Node]
          } else {
            vec![]
          }
        })
    };

    let next_sibling = |level: &mut u32, mut id| {
      while *level > 0 {
        let next = self.arena[id].next_sibling();
        if next.is_some() {
          return next;
        } else {
          id = self.arena[id].parent().unwrap();
          *level -= 1;
        }
      }
      None
    };
    let mut tab_indexs = vec![];
    let mut node = self.arena[scope_id].first_child();
    let mut level = 1;
    while let Some(id) = node {
      let tab_index = self.tab_index(id);
      if tab_index >= 0 {
        node_type(id)
          .into_iter()
          .for_each(|t| tab_indexs.push((tab_index, id, t)));
      }
      if !is_scope(id) {
        level += 1;
        node = self.arena[id].first_child().or_else(|| {
          level -= 1;
          next_sibling(&mut level, id)
        });
      } else {
        node = next_sibling(&mut level, id);
      }
    }
    tab_indexs.sort_by(|lh, rh| {
      if lh.0 == rh.0 {
        Ordering::Equal
      } else if lh.0 == 0 {
        Ordering::Greater
      } else if rh.0 == 0 {
        Ordering::Less
      } else {
        lh.0.cmp(&rh.0)
      }
    });
    if backward {
      tab_indexs.reverse();
    }
    tab_indexs
  }

  fn focus_step_in_scope(
    &self, scope_id: NodeId, node_id: Option<NodeId>, backward: bool,
  ) -> Option<NodeId> {
    let vec = self.collect_tab_index_in_scope(scope_id, backward);
    let idx = vec
      .iter()
      .position(move |(_, id, _)| Some(*id) == node_id)
      .map_or(0, |idx| idx + 1);

    for (_, id, focus_type) in &vec[idx..] {
      let next = if *focus_type == FocusType::Scope {
        self.focus_step_in_scope(*id, None, backward)
      } else {
        Some(*id)
      };
      if next.is_some() {
        return next;
      }
    }
    None
  }

  fn scope_id(&self, node_id: NodeId) -> Option<NodeId> { self.scope_list(node_id).next() }

  fn scope_list(&self, node_id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    node_id
      .ancestors(&self.arena)
      .skip(1)
      .filter(|n| self.assert_get(*n).has_focus_scope())
  }

  fn ignore_scope_id(&self, wid: WidgetId) -> Option<NodeId> {
    let wnd = self.window();

    let tree = wnd.tree();
    let node_id = wid
      .ancestors(tree)
      .find_map(|wid| self.node_ids.get(&wid).copied())?;

    self.scope_list(node_id).find(|id| {
      self
        .get(*id)
        .and_then(|n| n.wid)
        .filter(|wid| !wid.is_dropped(tree))
        .and_then(|wid| wid.query_ref::<FocusScope>(tree))
        .is_some_and(|s| s.skip_descendants)
    })
  }

  fn scope_property(&self, scope_id: Option<WidgetId>) -> FocusScope {
    let wnd = self.window();
    let tree = wnd.tree();
    scope_id
      .filter(|wid| !wid.is_dropped(tree))
      .and_then(|id| {
        id.query_ref::<FocusScope>(tree)
          .map(|s| s.clone())
      })
      .unwrap_or_default()
  }

  fn tab_index(&self, node_id: NodeId) -> i16 {
    let wnd = self.window();
    let tree = wnd.tree();
    self
      .get(node_id)
      .and_then(|n| n.wid)
      .filter(|wid| !wid.is_dropped(tree))
      .and_then(|wid| {
        wid
          .query_all_iter::<MixBuiltin>(tree)
          .find_map(|m| m.mix_flags().read().tab_index())
      })
      .unwrap_or_default()
  }

  fn insert_node(&mut self, parent: NodeId, node_id: NodeId, wid: WidgetId, arena: &WidgetTree) {
    enum TreePosition {
      BeforeSibling, // the new node is the sibling before current node
      SubTree,       // the new node is in the subtree of current node
      AfterSibling,  // the new node is the sibling after current node
      Skip,          // the node is not in the parent's sub-tree
    }

    fn locate_position(dst: &[WidgetId], base: &[WidgetId], arena: &WidgetTree) -> TreePosition {
      assert!(dst.len() > 1);
      let cnt = dst
        .iter()
        .rev()
        .zip(base.iter().rev())
        .take_while(|(wid1, wid2)| wid1 == wid2)
        .count();

      if dst.len() == cnt {
        return TreePosition::SubTree;
      } else if cnt == 0 {
        return TreePosition::Skip;
      }

      let parent = dst[dst.len() - cnt];
      let lh = dst[dst.len() - cnt - 1];
      let rh = base[base.len() - cnt - 1];

      for id in parent.children(arena) {
        if id == lh {
          return TreePosition::BeforeSibling;
        } else if id == rh {
          return TreePosition::AfterSibling;
        }
      }
      TreePosition::AfterSibling
    }

    fn collect_sub_ancestors(
      wid: WidgetId, pid: Option<WidgetId>, arena: &WidgetTree,
    ) -> Vec<WidgetId> {
      if wid.is_dropped(arena) {
        return vec![];
      }
      if let Some(pid) = pid {
        let mut arr: Vec<WidgetId> = wid
          .ancestors(arena)
          .take_while(|id| *id != pid)
          .collect();
        arr.push(pid);
        arr
      } else {
        wid.ancestors(arena).collect()
      }
    }

    let pwid = self.assert_get(parent).wid;
    let path = collect_sub_ancestors(wid, pwid, arena);

    let mut before_sibling = None;
    let mut afrer_sibling = None;
    let mut children = vec![];
    for id in parent.children(&self.arena) {
      let wid = self
        .arena
        .get(id)
        .and_then(|node| node.get().wid)
        .unwrap();
      let path2 = collect_sub_ancestors(wid, pwid, arena);

      match locate_position(&path, &path2, arena) {
        TreePosition::BeforeSibling => before_sibling = Some(id),
        TreePosition::SubTree => children.push(id),
        TreePosition::AfterSibling => afrer_sibling = Some(id),
        TreePosition::Skip => (),
      }

      if before_sibling.is_some() {
        break;
      }
    }

    if let Some(id) = before_sibling {
      id.insert_before(node_id, &mut self.arena);
    } else if let Some(id) = afrer_sibling {
      id.insert_after(node_id, &mut self.arena);
    } else {
      parent.append(node_id, &mut self.arena);
    }
    for child in children {
      node_id.append(child, &mut self.arena);
    }
  }

  fn get(&self, node_id: NodeId) -> Option<&FocusNodeInfo> {
    self.arena.get(node_id).map(|n| n.get())
  }

  fn assert_get(&self, node_id: NodeId) -> &FocusNodeInfo {
    self
      .get(node_id)
      .expect("focus not exists in the `tree`")
  }
}

impl FocusManager {
  pub fn focus_next_widget(&mut self, tree: &WidgetTree) {
    self.focus_move_circle(false);
    self.refresh_focus(tree);
  }

  pub fn focus_prev_widget(&mut self, tree: &WidgetTree) {
    self.focus_move_circle(true);
    self.refresh_focus(tree);
  }

  /// Attempts to focus the specified widget, returning the actual focused
  /// widget ID on success.
  ///
  /// Returns `None` if:
  /// - The widget is in an ignored focus scope
  /// - The widget doesn't exist in the tree
  /// - The widget isn't focusable through normal navigation
  pub fn try_focus(&mut self, wid: WidgetId, tree: &WidgetTree) -> Option<WidgetId> {
    if self.ignore_scope_id(wid).is_some() {
      return None;
    }

    let node = self.node_ids.get(&wid)?;
    let info = self.get(*node)?;

    let id = if info.has_focus_scope() {
      let scope = self.scope_property(info.wid);
      if info.has_focus_node() && !scope.skip_host {
        info.wid
      } else if !scope.skip_descendants {
        self
          .focus_step_in_scope(*node, None, false)
          .and_then(|id| self.assert_get(id).wid)
      } else {
        None
      }
    } else {
      info.wid
    };
    self.request_focus_to(id);
    self.refresh_focus(tree);

    self.focusing()
  }

  pub fn focus(&mut self, wid: WidgetId, tree: &WidgetTree) {
    self.request_focus_to(Some(wid));
    self.refresh_focus(tree);
  }

  pub fn blur(&mut self, tree: &WidgetTree) {
    self.request_focus_to(None);
    self.refresh_focus(tree);
  }

  pub(crate) fn blur_on_dispose(&mut self) { self.change_focusing_to(None); }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focusing }

  pub fn refresh_focus(&mut self, tree: &WidgetTree) {
    let new_focus = self.next_focus(tree);
    if self.focus_widgets.first() != new_focus.as_ref() {
      self.change_focusing_to(new_focus);
    }
  }

  // When focus_to is Some(wid), wid requests focus, which delays refreshing the
  // focus, because wid may be a newly added widget in init. But you can call
  // refresh_focus manually to force a refresh. Conversely, if focus_to is set
  // to None and the focused widget requests blur, it will refresh focus
  // immediately because the widget may be in a disposed state and the widget
  // will be removed soon.
  pub(crate) fn request_focus_to(&mut self, focus_to: Option<WidgetId>) {
    self.request_focusing = Some(focus_to);
  }

  fn change_focusing_to(&mut self, node: Option<WidgetId>) -> Option<WidgetId> {
    let wnd = self.window();
    let tree = wnd.tree();

    // dispatch blur event
    if let Some(wid) = self.focusing() {
      wnd.add_delay_event(DelayEvent::Blur(wid));
    };

    let old = self
      .focus_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(tree))
      .copied();

    // bubble focus out
    if let Some(old) = old {
      let ancestor = node.and_then(|w| w.lowest_common_ancestor(old, tree));
      wnd.add_delay_event(DelayEvent::FocusOut { bottom: old, up: ancestor });
    };

    if let Some(new) = node {
      wnd.add_delay_event(DelayEvent::Focus(new));
      let ancestor = old.and_then(|o| o.lowest_common_ancestor(new, tree));
      wnd.add_delay_event(DelayEvent::FocusIn { bottom: new, up: ancestor });
    }

    self.focus_widgets = node.map_or(vec![], |wid| wid.ancestors(tree).collect::<Vec<_>>());
    self.focusing = node;
    old
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;

  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn two_auto_focus() {
    reset_test_env!();

    // two auto focus widget
    let size = Size::zero();
    let widget = fn_widget! {
      @MockMulti  {
        @MockBox { size, auto_focus: true, }
        @MockBox { size, auto_focus: true, }
      }
    };

    let wnd = TestWindow::new(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let tree = wnd.tree();

    focus_mgr.refresh_focus(tree);

    let id = tree.content_root().first_child(tree);
    assert!(id.is_some());
    assert_eq!(focus_mgr.focusing(), id);
  }

  #[test]
  fn on_auto_focus() {
    reset_test_env!();
    // one auto focus widget
    let size = Size::zero();
    let widget = fn_widget! {
      @MockMulti {
        @MockBox { size }
        @MockBox { size, auto_focus: true}
      }
    };

    let wnd = TestWindow::new(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let tree = wnd.tree();

    let id = tree
      .content_root()
      .first_child(tree)
      .and_then(|p| p.next_sibling(tree));
    assert!(id.is_some());
    focus_mgr.refresh_focus(tree);
    assert_eq!(focus_mgr.focusing(), id);
  }

  #[test]
  fn tab_index() {
    reset_test_env!();

    let size = Size::zero();
    let widget = fn_widget! {
      @MockMulti {
        @MockBox { size, tab_index: -1i16, }
        @MockBox { size, tab_index: 0i16, }
        @MockBox { size, tab_index: 1i16, auto_focus: true}
        @MockBox { size, tab_index: 2i16, }
        @MockMulti { tab_index: 4i16, @MockBox { size, tab_index: 3i16, } }
        @MockBox { size, tab_index: 0i16 }
      }
    };

    let wnd = TestWindow::new(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();

    let tree = wnd.tree();

    focus_mgr.refresh_focus(tree);

    let negative = tree.content_root().first_child(tree).unwrap();
    let id0 = negative.next_sibling(tree).unwrap();
    let id1 = id0.next_sibling(tree).unwrap();
    let id2 = id1.next_sibling(tree).unwrap();
    let id4 = id2.next_sibling(tree).unwrap();
    let id3 = id4.first_child(tree).unwrap();
    let id0_0 = id4.next_sibling(tree).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id2));
      focus_mgr.focus_next_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id3));
      focus_mgr.focus_next_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id4));
      focus_mgr.focus_next_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id0));
      focus_mgr.focus_next_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id0_0));
      focus_mgr.focus_next_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id1));

      // previous focus sequential

      focus_mgr.focus_prev_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id0_0));
      focus_mgr.focus_prev_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id0));
      focus_mgr.focus_prev_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id4));
      focus_mgr.focus_prev_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id3));
      focus_mgr.focus_prev_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id2));
      focus_mgr.focus_prev_widget(tree);
      assert_eq!(focus_mgr.focusing(), Some(id1));
    }
  }

  #[test]
  fn focus_event() {
    reset_test_env!();

    #[derive(Debug, Default, Clone)]
    struct EmbedFocus {
      log: Sc<RefCell<Vec<&'static str>>>,
    }

    impl Compose for EmbedFocus {
      fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
          @MockBox {
            size: INFINITY_SIZE,
            on_focus: move |_| {
              $this.log.borrow_mut().push("focus parent");
            },
            on_blur: move |_| {
              $this.log.borrow_mut().push("blur parent");
            },
            on_focus_in: move |_| {
              $this.log.borrow_mut().push("focusin parent");
            },
            on_focus_out: move |_| {
              $this.log.borrow_mut().push("focusout parent");
            },
            @MockBox {
              size: Size::zero(),
              on_focus: move |_| {
                $this.log.borrow_mut().push("focus child");
              },
              on_blur: move |_| {
                $this.log.borrow_mut().push("blur child");
              },
              on_focus_in: move |_| {
                $this.log.borrow_mut().push("focusin child");
              },
              on_focus_out: move |_| {
                $this.log.borrow_mut().push("focusout child");
              },
            }
          }
        }
        .into_widget()
      }
    }

    let widget = EmbedFocus::default();
    let log: Sc<RefCell<Vec<&str>>> = widget.log.clone();
    let mut wnd = TestWindow::new(fn_widget! {
      widget.clone()
    });

    let parent = wnd.tree().content_root();
    let child = parent.first_child(wnd.tree()).unwrap();

    wnd
      .focus_mgr
      .borrow_mut()
      .refresh_focus(wnd.tree());
    wnd
      .focus_mgr
      .borrow_mut()
      .focus(child, wnd.tree());
    wnd.draw_frame();
    assert_eq!(&*log.borrow(), &["focus child", "focusin child", "focusin parent"]);
    log.borrow_mut().clear();

    wnd
      .focus_mgr
      .borrow_mut()
      .focus_next_widget(wnd.tree());
    wnd.run_frame_tasks();
    assert_eq!(&*log.borrow(), &["blur child", "focusout child", "focus parent",]);
    log.borrow_mut().clear();

    wnd.focus_mgr.borrow_mut().blur(wnd.tree());
    wnd.run_frame_tasks();
    assert_eq!(&*log.borrow(), &["blur parent", "focusout parent",]);
  }

  #[test]
  fn dynamic_focus() {
    reset_test_env!();

    let visible = Stateful::new(Some(()));
    let c_visible = visible.clone_writer();
    let w = fn_widget! {
      @MockMulti{
        @ { pipe! {
          $visible.map(|_| fn_widget! {
            @MockBox {
              size: Size::default(),
              on_tap: move |_| {},
            }
          })}
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    let focus_id = wnd.focus_mgr.borrow_mut().focusing();

    wnd.draw_frame();

    *c_visible.write() = None;
    wnd.draw_frame();

    *c_visible.write() = Some(());
    wnd.draw_frame();

    assert_eq!(wnd.focus_mgr.borrow().focusing(), focus_id);
  }

  #[test]
  fn scope_node_request_focus() {
    reset_test_env!();

    let w = fn_widget! {
      @MockMulti{
        @MockBox{
          size: Size::zero(),
          on_key_down: move |_| {}
        }
        @FocusScope {
          @MockBox{
            size: Size::zero(),
            @MockBox{
              size: Size::zero(),
              on_key_down: move |_| {}
            }
          }
        }
        @MockBox{
          size: Size::zero(),
          on_key_down: move |_| {}
        }
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let tree = wnd.tree();

    let first_box = tree.content_root().first_child(tree);
    let focus_scope = first_box.unwrap().next_sibling(tree);
    focus_mgr.request_focus_to(focus_scope);

    let inner_box = focus_scope.unwrap().first_child(tree);
    focus_mgr.refresh_focus(tree);
    assert_eq!(focus_mgr.focusing(), inner_box);
  }

  #[test]
  fn remove_focused_widget() {
    reset_test_env!();
    let (input, input_writer) = split_value(String::default());
    let (focused, focused_writer) = split_value(true);
    let w = fn_widget! {
      @MockBox{
        size: Size::new(20., 20.),
        @ {
          pipe!(*$focused).map(move |v| v.then(move || fn_widget!{
            @MockBox {
            auto_focus: true,
            on_chars: move |e| $input_writer.write().push_str(&e.chars),
            size: Size::new(10., 10.),
            }
          }))
        }
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.processes_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "hello");

    *focused_writer.write() = false;
    wnd.draw_frame();
    wnd.processes_receive_chars("has no receiver".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "hello");

    *focused_writer.write() = true;
    wnd.draw_frame();
    wnd.processes_receive_chars(" ribir".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "hello ribir");
  }

  #[test]
  fn multi_focused_update() {
    reset_test_env!();
    let (input, input_writer) = split_value(String::default());
    let (active_idx, active_idx_writer) = split_value(0);
    let w = fn_widget! {
      @MockMulti{
        @ {
          (0..4).map(move |i| {
            pipe! (*$active_idx).map(move |idx| fn_widget!{
              @MockBox {
                auto_focus: i == idx,
                on_chars: move |e| if idx == 2 { $input_writer.write().push_str(&e.chars) },
                size: Size::new(10., 10.),
              }
            })
          }).collect::<Vec<_>>()
        }
      }
    };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    wnd.processes_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "");

    *active_idx_writer.write() += 1;
    wnd.draw_frame();
    wnd.processes_receive_chars("ribir".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "");

    *active_idx_writer.write() += 1;
    wnd.draw_frame();
    wnd.processes_receive_chars("nice to see you".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "nice to see you");

    *active_idx_writer.write() += 1;
    wnd.draw_frame();
    wnd.processes_receive_chars("Bye-Bye".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "nice to see you");
    wnd.draw_frame();
  }
}
