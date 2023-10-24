use crate::{
  prelude::*,
  widget_tree::TreeArena,
  window::{DelayEvent, WindowId},
};

use indextree::{Arena, NodeId};
use std::{
  cmp::Ordering,
  rc::{Rc, Weak},
};

#[derive(Debug)]
pub(crate) struct FocusManager {
  /// store current focusing node, and its position in tab_orders.
  focusing: Option<WidgetId>,
  request_focusing: Option<Option<WidgetId>>,
  focus_widgets: Vec<WidgetId>,
  node_ids: ahash::HashMap<WidgetId, NodeId>,
  arena: Arena<FocusNodeInfo>,
  root: NodeId,
  wnd: Weak<Window>,
}

pub struct FocusHandle {
  wid: WidgetId,
  wnd_id: WindowId,
}

impl FocusHandle {
  pub(crate) fn request_focus(&self) {
    if let Some(wnd) = AppCtx::get_window(self.wnd_id) {
      wnd.focus_mgr.borrow_mut().request_focusing = Some(Some(self.wid));
    }
  }

  pub(crate) fn unfocus(&self) {
    if let Some(wnd) = AppCtx::get_window(self.wnd_id) {
      if wnd.focus_mgr.borrow().focusing == Some(self.wid) {
        wnd.focus_mgr.borrow_mut().request_focusing = Some(None);
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
  fn new(wid: WidgetId) -> Self {
    FocusNodeInfo {
      scope_cnt: 0,
      node_cnt: 0,
      wid: Some(wid),
    }
  }

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
  pub(crate) fn new() -> Self {
    let mut arena = Arena::new();
    let root = arena.new_node(FocusNodeInfo { node_cnt: 0, scope_cnt: 1, wid: None });
    Self {
      wnd: Weak::new(),
      focus_widgets: Vec::new(),
      request_focusing: None,
      focusing: None,
      node_ids: ahash::HashMap::default(),
      arena,
      root,
    }
  }

  pub(crate) fn init(&mut self, wnd: Weak<Window>) { self.wnd = wnd }

  pub(crate) fn window(&self) -> Rc<Window> {
    self
      .wnd
      .upgrade()
      .expect("The window of `FocusManager` has already dropped.")
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
      let arena = &wnd.widget_tree.borrow().arena;
      let mut it = wid.ancestors(arena).skip(1);
      let parent = it
        .find_map(|id| self.node_ids.get(&id))
        .unwrap_or(&self.root);
      self.insert_node(*parent, node_id, wid, arena);
    }

    if auto_focus
      && focus_type == FocusType::Node
      && self.focusing.is_none()
      && self.request_focusing.is_none()
    {
      self.request_focusing = Some(Some(wid));
    }
  }

  pub(crate) fn focus_handle(&self, wid: WidgetId) -> FocusHandle {
    FocusHandle { wid, wnd_id: self.window().id() }
  }

  pub(crate) fn remove_focus_node(&mut self, wid: WidgetId, focus_type: FocusType) {
    let Some(id) = self.node_ids.get(&wid).cloned() else {
      return;
    };

    let node = self.arena[id].get_mut();
    node.remove_focus(focus_type);

    if Some(wid) == self.focusing && !node.has_focus_node() {
      self.request_focusing = Some(None);
    }

    if node.is_empty() {
      id.remove(&mut self.arena);
      self.node_ids.remove(&wid);
    }
  }

  pub fn refresh(&mut self) {
    let Some(focusing) = self.request_focusing.take() else {
      return;
    };
    if focusing.is_none() {
      self.focusing = None;
      return;
    }

    let focusing = focusing.filter(|node_id| self.ignore_scope_id(*node_id).is_none());
    let focus_node = focusing.and_then(|wid| self.node_ids.get(&wid));
    let info = focus_node.and_then(|id: &NodeId| self.get(*id));

    let focus_to = if let Some(node) = info {
      if node.has_focus_scope() {
        let scope = self.scope_property(node.wid);
        if node.has_focus_node() && scope.can_focus {
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
    self.focusing = focus_to;
  }

  fn focus_move_circle(&mut self, backward: bool) {
    let has_focus = self.focusing.is_some();
    let mut wid = self.focus_step(self.focusing, backward);
    if wid.is_none() && has_focus {
      wid = self.focus_step(wid, backward);
    }
    self.request_focusing = Some(wid);
  }

  fn focus_step(&mut self, focusing: Option<WidgetId>, backward: bool) -> Option<WidgetId> {
    let mut node_id = focusing.and_then(|id| self.node_ids.get(&id)).copied();
    let mut scope_id = node_id.and_then(|id| self.scope_id(id)).or(Some(self.root));
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
    &self,
    scope_id: NodeId,
    backward: bool,
  ) -> Vec<(i16, NodeId, FocusType)> {
    let scope_tab_type = |id, has_focus_node: bool| {
      let mut v = vec![];
      let node = self.scope_property(id);

      if has_focus_node && node.can_focus {
        v.push(FocusType::Node);
      }
      if !node.skip_descendants {
        v.push(FocusType::Scope);
      }

      v
    };
    let is_scope = |id| self.assert_get(id).has_focus_scope();
    let node_type = |id| {
      self.arena.get(id).map(|n| n.get()).map_or(vec![], |node| {
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
    &self,
    scope_id: NodeId,
    node_id: Option<NodeId>,
    backward: bool,
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

    let arena = &wnd.widget_tree.borrow().arena;
    let node_id = wid
      .ancestors(arena)
      .find_map(|wid| self.node_ids.get(&wid).copied())?;

    self.scope_list(node_id).find(|id| {
      let mut has_ignore = false;
      self.get(*id).and_then(|n| n.wid).map(|wid| {
        wid.get(arena)?.query_most_inside(|s: &FocusScope| {
          has_ignore = s.skip_descendants;
          !has_ignore
        })
      });
      has_ignore
    })
  }

  fn scope_property(&self, scope_id: Option<WidgetId>) -> FocusScope {
    let wnd = self.window();
    let tree = wnd.widget_tree.borrow();
    scope_id
      .and_then(|id| id.get(&tree.arena))
      .and_then(|r| r.query_most_inside(|s: &FocusScope| s.clone()))
      .unwrap_or_default()
  }

  fn tab_index(&self, node_id: NodeId) -> i16 {
    let wnd = self.window();

    let get_index = || {
      let wid = self.get(node_id)?.wid?;
      let tree = wnd.widget_tree.borrow();
      let r = wid.get(&tree.arena)?;
      r.query_most_outside(|s: &FocusNode| s.tab_index)
    };

    get_index().unwrap_or(0)
  }

  fn insert_node(&mut self, parent: NodeId, node_id: NodeId, wid: WidgetId, arena: &TreeArena) {
    enum TreePosition {
      BeforeSibling,
      SubTree,
      AfterSibling,
    }

    fn locate_position(
      dst: &Vec<WidgetId>,
      base: &Vec<WidgetId>,
      arena: &TreeArena,
    ) -> TreePosition {
      assert!(dst.len() > 1 && base.len() > 1);
      let cnt = dst
        .iter()
        .rev()
        .zip(base.iter().rev())
        .take_while(|(wid1, wid2)| wid1 == wid2)
        .count();

      if dst.len() == cnt {
        return TreePosition::SubTree;
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
      wid: WidgetId,
      pid: Option<WidgetId>,
      arena: &TreeArena,
    ) -> Vec<WidgetId> {
      if let Some(pid) = pid {
        let mut arr: Vec<WidgetId> = wid.ancestors(arena).take_while(|id| *id != pid).collect();
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
      let wid = self.arena.get(id).and_then(|node| node.get().wid).unwrap();
      let path2 = collect_sub_ancestors(wid, pwid, arena);

      match locate_position(&path, &path2, arena) {
        TreePosition::BeforeSibling => before_sibling = Some(id),
        TreePosition::SubTree => children.push(id),
        TreePosition::AfterSibling => afrer_sibling = Some(id),
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
    self.get(node_id).expect("focus not exists in the `tree`")
  }
}

impl FocusManager {
  pub fn focus_next_widget(&mut self) {
    self.focus_move_circle(false);
    self.refresh_focus();
  }

  pub fn focus_prev_widget(&mut self) {
    self.focus_move_circle(true);
    self.refresh_focus();
  }

  pub fn focus(&mut self, wid: WidgetId) { self.change_focusing_to(Some(wid)); }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(&mut self) -> Option<WidgetId> { self.change_focusing_to(None) }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focusing }

  pub fn refresh_focus(&mut self) {
    self.refresh();
    if self.focus_widgets.first() != self.focusing.as_ref() {
      self.change_focusing_to(self.focusing);
    }
  }

  fn change_focusing_to(&mut self, node: Option<WidgetId>) -> Option<WidgetId> {
    let wnd = self.window();
    let tree = wnd.widget_tree.borrow();

    // dispatch blur event
    if let Some(wid) = self.focusing() {
      wnd.add_delay_event(DelayEvent::Blur(wid));
    };

    let old = self
      .focus_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(&tree.arena))
      .copied();

    // bubble focus out
    if let Some(old) = old {
      let ancestor = node.and_then(|w| w.common_ancestors(old, &tree.arena).next());
      wnd.add_delay_event(DelayEvent::FocusOut { bottom: old, up: ancestor });
    };

    if let Some(new) = node {
      wnd.add_delay_event(DelayEvent::Focus(new));
      let ancestor = old.and_then(|o| o.common_ancestors(new, &tree.arena).next());
      wnd.add_delay_event(DelayEvent::FocusIn { bottom: new, up: ancestor });
    }

    self.focus_widgets = node.map_or(vec![], |wid| wid.ancestors(&tree.arena).collect::<Vec<_>>());
    self.focusing = node;
    old
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};
  use std::{cell::RefCell, rc::Rc};

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
    let tree = wnd.widget_tree.borrow();

    focus_mgr.refresh_focus();

    let id = tree.root().first_child(&tree.arena);
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
    let tree = wnd.widget_tree.borrow();

    let id = tree
      .root()
      .first_child(&tree.arena)
      .and_then(|p| p.next_sibling(&tree.arena));
    assert!(id.is_some());
    focus_mgr.refresh_focus();
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
    let widget_tree = wnd.widget_tree.borrow();

    focus_mgr.refresh_focus();

    let arena = &widget_tree.arena;
    let negative = widget_tree.root().first_child(arena).unwrap();
    let id0 = negative.next_sibling(arena).unwrap();
    let id1 = id0.next_sibling(arena).unwrap();
    let id2 = id1.next_sibling(arena).unwrap();
    let id4 = id2.next_sibling(arena).unwrap();
    let id3 = id4.first_child(arena).unwrap();
    let id0_0 = id4.next_sibling(arena).unwrap();

    {
      // next focus sequential
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id2));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id3));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id4));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id0));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id0_0));
      focus_mgr.focus_next_widget();
      assert_eq!(focus_mgr.focusing(), Some(id1));

      // previous focus sequential

      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id0_0));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id0));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id4));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id3));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id2));
      focus_mgr.focus_prev_widget();
      assert_eq!(focus_mgr.focusing(), Some(id1));
    }
  }

  #[test]
  fn focus_event() {
    reset_test_env!();

    #[derive(Debug, Default)]
    struct EmbedFocus {
      log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Compose for EmbedFocus {
      fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
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
      }
    }

    let widget = EmbedFocus::default();
    let log = widget.log.clone();
    let mut wnd = TestWindow::new(fn_widget!(widget));
    let tree = wnd.widget_tree.borrow();
    let parent = tree.root();
    let child = parent.first_child(&tree.arena).unwrap();
    drop(tree);

    wnd.focus_mgr.borrow_mut().refresh_focus();
    wnd.focus_mgr.borrow_mut().focus(child);
    wnd.draw_frame();
    assert_eq!(
      &*log.borrow(),
      &["focus child", "focusin child", "focusin parent"]
    );
    log.borrow_mut().clear();

    wnd.focus_mgr.borrow_mut().focus(parent);
    wnd.run_frame_tasks();
    assert_eq!(
      &*log.borrow(),
      &["blur child", "focusout child", "focus parent",]
    );
    log.borrow_mut().clear();

    wnd.focus_mgr.borrow_mut().blur();
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
          $visible.map(|_| @MockBox {
            size: Size::default(),
            on_tap: move |_| {},
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
    let tree = wnd.widget_tree.borrow();

    let first_box = tree.root().first_child(&tree.arena);
    let focus_scope = first_box.unwrap().next_sibling(&tree.arena);
    focus_mgr.request_focusing = Some(focus_scope);

    let inner_box = focus_scope.unwrap().first_child(&tree.arena);
    focus_mgr.refresh_focus();
    assert_eq!(focus_mgr.focusing(), inner_box);
  }
}
