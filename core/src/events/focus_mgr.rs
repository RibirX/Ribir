use std::{cmp::Ordering, ptr::NonNull};

use indextree::{Arena, NodeId};

use crate::{
  prelude::*,
  window::{DelayEvent, WindowId},
};

/// The FocusEvent interface represents focus-related events, including focus,
/// blur, focusin, and focusout.
pub struct FocusEvent {
  pub common: CommonEvent,
  pub reason: FocusReason,
}

impl_common_event_deref!(FocusEvent);

/// Represents the source interaction that caused a UI element to gain focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FocusReason {
  /// Keyboard navigation (Tab/Arrow keys)
  Keyboard,
  /// Pointer interaction (mouse click/touch tap)
  Pointer,
  /// Automatic focus assignment during window loaded
  AutoFocus,
  /// Programmatic focus or unspecified source,
  Other,
}

#[derive(Debug)]
pub(crate) struct FocusManager {
  /// store current focusing node, and its position in tab_orders.
  focusing: Option<WidgetId>,
  frame_auto_focus: Vec<WidgetId>,
  focus_widgets: Vec<WidgetId>,
  node_ids: ahash::HashMap<WidgetId, NodeId>,
  arena: Arena<FocusNodeInfo>,
  root: NodeId,
  wnd_id: WindowId,
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

impl FocusReason {
  pub fn from_u8(reason: u8) -> FocusReason {
    match reason {
      0 => FocusReason::Keyboard,
      1 => FocusReason::Pointer,
      2 => FocusReason::AutoFocus,
      3 => FocusReason::Other,
      _ => unreachable!(),
    }
  }
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

  fn find_real_focus_widget(&mut self, focus: WidgetId) -> Option<WidgetId> {
    if self.ignore_scope_id(focus).is_some() {
      return None;
    }

    let focus_node = self.node_ids.get(&focus)?;
    let info = self.get(*focus_node)?;
    if info.has_focus_scope() {
      let scope = self.scope_property(info.wid);
      if info.has_focus_node() && !scope.skip_host {
        info.wid
      } else if !scope.skip_descendants {
        self
          .focus_step_in_scope(*focus_node, None, false)
          .and_then(|id| self.assert_get(id).wid)
      } else {
        None
      }
    } else {
      info.wid
    }
  }

  fn focus_move_circle(&mut self, backward: bool) -> Option<WidgetId> {
    let has_focus = self.focusing.is_some();
    let mut wid = self.focus_step(self.focusing, backward);
    if wid.is_none() && has_focus {
      wid = self.focus_step(wid, backward);
    }

    wid
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
  /// Attempts to move focus to the next widget, returning the actual focused
  /// widget ID on success.
  pub fn focus_next_widget(&mut self, reason: FocusReason) -> Option<WidgetId> {
    self
      .focus_move_circle(false)
      .and_then(|wid| self.focus(wid, reason))
  }

  /// Attempts to move focus to the previous widget, returning the actual
  /// focused widget ID on success.
  pub fn focus_prev_widget(&mut self, reason: FocusReason) -> Option<WidgetId> {
    self
      .focus_move_circle(true)
      .and_then(|wid| self.focus(wid, reason))
  }

  /// Attempts to focus the specified widget, returning the actual focused
  /// widget ID on success.
  ///
  /// Returns `None` if:
  /// - The widget is in an ignored focus scope
  /// - The widget doesn't exist in the tree
  /// - The widget isn't focusable through normal navigation
  pub fn focus(&mut self, wid: WidgetId, reason: FocusReason) -> Option<WidgetId> {
    let focus = self.find_real_focus_widget(wid)?;
    if self.focus_widgets.first() != Some(&focus) {
      self.change_focusing_to(Some(focus), reason);
      Some(focus)
    } else {
      None
    }
  }

  /// Blurs the current focused widget, and returns the previous focused.
  pub fn blur(&mut self, reason: FocusReason) -> Option<WidgetId> {
    self.change_focusing_to(None, reason)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focusing }

  pub fn on_widget_tree_update(&mut self, tree: &WidgetTree) {
    let autos = self
      .frame_auto_focus
      .drain(..)
      .find(|wid| !wid.is_dropped(tree))
      .or(self.focusing);

    if let Some(focus) = autos {
      self.focus(focus, FocusReason::AutoFocus);
    }
  }

  fn change_focusing_to(
    &mut self, node: Option<WidgetId>, reason: FocusReason,
  ) -> Option<WidgetId> {
    let wnd = self.window();
    let tree = wnd.tree();

    // dispatch blur event
    if let Some(wid) = self.focusing() {
      wnd.add_delay_event(DelayEvent::Blur { id: wid, reason });
    };

    let old = self
      .focus_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(tree))
      .copied();

    // bubble focus out
    if let Some(old) = old {
      let ancestor = node.and_then(|w| w.lowest_common_ancestor(old, tree));
      wnd.add_delay_event(DelayEvent::FocusOut { bottom: old, up: ancestor, reason });
    };

    if let Some(new) = node {
      wnd.add_delay_event(DelayEvent::Focus { id: new, reason });
      let ancestor = old.and_then(|o| o.lowest_common_ancestor(new, tree));
      wnd.add_delay_event(DelayEvent::FocusIn { bottom: new, up: ancestor, reason });
    }

    self.focus_widgets = node.map_or(vec![], |wid| wid.ancestors(tree).collect::<Vec<_>>());
    self.focusing = node;
    old
  }
}

impl FocusEvent {
  pub(crate) fn new(wid: WidgetId, reason: FocusReason, tree: NonNull<WidgetTree>) -> Self {
    let common = CommonEvent::new(wid, tree);
    Self { common, reason }
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

    let wnd = TestWindow::from_widget(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let tree = wnd.tree();
    focus_mgr.on_widget_tree_update(tree);

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

    let wnd = TestWindow::from_widget(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let tree = wnd.tree();

    let id = tree
      .content_root()
      .first_child(tree)
      .and_then(|p| p.next_sibling(tree));
    assert!(id.is_some());
    focus_mgr.focus(id.unwrap(), FocusReason::Other);
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

    let wnd = TestWindow::from_widget(widget);
    let mut focus_mgr = wnd.focus_mgr.borrow_mut();

    let tree = wnd.tree();
    focus_mgr.on_widget_tree_update(tree);

    let negative = tree.content_root().first_child(tree).unwrap();
    let id0 = negative.next_sibling(tree).unwrap();
    let id1 = id0.next_sibling(tree).unwrap();
    let id2 = id1.next_sibling(tree).unwrap();
    let id4 = id2.next_sibling(tree).unwrap();
    let id3 = id4.first_child(tree).unwrap();
    let id0_0 = id4.next_sibling(tree).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id2));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id3));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id4));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id0));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id0_0));
      focus_mgr.focus_next_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id1));

      // previous focus sequential

      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id0_0));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id0));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id4));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id3));
      focus_mgr.focus_prev_widget(FocusReason::Other);
      assert_eq!(focus_mgr.focusing(), Some(id2));
      focus_mgr.focus_prev_widget(FocusReason::Other);
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
              $read(this).log.borrow_mut().push("focus parent");
            },
            on_blur: move |_| {
              $read(this).log.borrow_mut().push("blur parent");
            },
            on_focus_in: move |_| {
              $read(this).log.borrow_mut().push("focusin parent");
            },
            on_focus_out: move |_| {
              $read(this).log.borrow_mut().push("focusout parent");
            },
            @MockBox {
              size: Size::zero(),
              on_focus: move |_| {
                $read(this).log.borrow_mut().push("focus child");
              },
              on_blur: move |_| {
                $read(this).log.borrow_mut().push("blur child");
              },
              on_focus_in: move |_| {
                $read(this).log.borrow_mut().push("focusin child");
              },
              on_focus_out: move |_| {
                $read(this).log.borrow_mut().push("focusout child");
              },
            }
          }
        }
        .into_widget()
      }
    }

    let widget = EmbedFocus::default();
    let log: Sc<RefCell<Vec<&str>>> = widget.log.clone();
    let wnd = TestWindow::from_widget(fn_widget! {
      widget.clone()
    });

    let parent = wnd.tree().content_root();
    let child = parent.first_child(wnd.tree()).unwrap();

    wnd
      .focus_mgr
      .borrow_mut()
      .on_widget_tree_update(wnd.tree());
    wnd
      .focus_mgr
      .borrow_mut()
      .focus(child, FocusReason::Other);
    wnd.draw_frame();
    assert_eq!(&*log.borrow(), &["focus child", "focusin child", "focusin parent"]);
    log.borrow_mut().clear();

    wnd
      .focus_mgr
      .borrow_mut()
      .focus_next_widget(FocusReason::Other);
    wnd.run_frame_tasks();
    assert_eq!(&*log.borrow(), &["blur child", "focusout child", "focus parent",]);
    log.borrow_mut().clear();

    wnd
      .focus_mgr
      .borrow_mut()
      .blur(FocusReason::Other);
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
          $read(visible).map(|_| fn_widget! {
            @MockBox {
              size: Size::default(),
              on_tap: move |_| {},
            }
          })}
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
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
    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();

    let mut focus_mgr = wnd.focus_mgr.borrow_mut();
    let tree = wnd.tree();

    let first_box = tree.content_root().first_child(tree);
    let focus_scope = first_box.unwrap().next_sibling(tree);
    let inner_box = focus_scope.unwrap().first_child(tree);
    focus_mgr.focus(inner_box.unwrap(), FocusReason::Other);
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
          pipe!(*$read(focused)).map(move |v| v.then(move || fn_widget!{
            @MockBox {
            auto_focus: true,
            on_chars: move |e| $write(input_writer).push_str(&e.chars),
            size: Size::new(10., 10.),
            }
          }))
        }
      }
    };
    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.process_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "hello");

    *focused_writer.write() = false;
    wnd.draw_frame();
    wnd.process_receive_chars("has no receiver".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "hello");

    *focused_writer.write() = true;
    wnd.draw_frame();
    wnd.process_receive_chars(" ribir".into());
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
            pipe! (*$read(active_idx)).map(move |idx| fn_widget!{
              @MockBox {
                auto_focus: i == idx,
                on_chars: move |e| if idx == 2 { $write(input_writer).push_str(&e.chars) },
                size: Size::new(10., 10.),
              }
            })
          }).collect::<Vec<_>>()
        }
      }
    };
    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    wnd.process_receive_chars("hello".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "");

    *active_idx_writer.write() += 1;
    wnd.draw_frame();
    wnd.process_receive_chars("ribir".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "");

    *active_idx_writer.write() += 1;
    wnd.draw_frame();
    wnd.process_receive_chars("nice to see you".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "nice to see you");

    *active_idx_writer.write() += 1;
    wnd.draw_frame();
    wnd.process_receive_chars("Bye-Bye".into());
    wnd.draw_frame();
    assert_eq!(*input.read(), "nice to see you");
    wnd.draw_frame();
  }

  #[test]
  fn focus_reason_from_u8() {
    assert_eq!(FocusReason::from_u8(0), FocusReason::Keyboard);
    assert_eq!(FocusReason::from_u8(1), FocusReason::Pointer);
    assert_eq!(FocusReason::from_u8(2), FocusReason::AutoFocus);
    assert_eq!(FocusReason::from_u8(3), FocusReason::Other);
  }

  #[test]
  fn focus_reason_to_u8() {
    assert_eq!(FocusReason::Keyboard as u8, 0);
    assert_eq!(FocusReason::Pointer as u8, 1);
    assert_eq!(FocusReason::AutoFocus as u8, 2);
    assert_eq!(FocusReason::Other as u8, 3);
  }

  #[test]
  #[should_panic]
  fn focus_reason_unreachable() { FocusReason::from_u8(4); }

  #[test]
  fn track_focus_reason() {
    reset_test_env!();
    let (reason, w_reason) = split_value(FocusReason::Other);
    let f = fn_widget! {
      let mut w = @Container {
        auto_focus: true,
        size: Size::splat(100.)
      };
      let u = watch!(*$read(w.focus_changed_reason()))
        .subscribe(move |v| *$write(w_reason) = v);
      @(w) {
        on_disposed: move |_| u.unsubscribe()
      }
    };

    let wnd = TestWindow::from_widget(f);

    wnd.draw_frame();
    assert_eq!(*reason.read(), FocusReason::AutoFocus);

    wnd
      .focus_mgr
      .borrow_mut()
      .blur(FocusReason::Other);
    wnd.draw_frame();
    assert_eq!(*reason.read(), FocusReason::Other);

    wnd.process_mouse_press(Box::new(DummyDeviceId), MouseButtons::PRIMARY);
    wnd.draw_frame();
    assert_eq!(*reason.read(), FocusReason::Pointer);
  }

  #[test]
  fn dynamic_focus_node() {
    reset_test_env!();

    let widget = fn_widget! {
      let mut m = @MockBox {
        tab_index: 0i16,
        size: Size::default(),
      };
      let mut m = @(m) { tab_index: 0i16, };
      @(m) { tab_index: 0i16 }
    };

    let wnd = TestWindow::from_widget(widget);
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
