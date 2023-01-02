use crate::{prelude::*, widget_tree::TreeArena};

use indextree::{Arena, NodeId};
use std::{
  cell::RefCell,
  cmp::Ordering,
  collections::{HashMap, HashSet},
  rc::Rc,
};

use super::dispatcher::Dispatcher;

#[derive(Debug)]
pub(crate) struct FocusManager {
  /// store current focusing node, and its position in tab_orders.
  focusing: Option<WidgetId>,
  node_ids: HashMap<WidgetId, NodeId>,
  arena: Arena<FocusNodeInfo>,
  root: NodeId,
}

pub struct FocustHandle {
  wid: WidgetId,
  mgr: Rc<RefCell<FocusManager>>,
}

impl FocustHandle {
  pub(crate) fn request_focus(&self) { self.mgr.borrow_mut().focus_to(Some(self.wid)); }

  pub(crate) fn unfocus(&self) {
    if self.mgr.borrow().focusing == Some(self.wid) {
      self.mgr.borrow_mut().focus_to(None);
    }
  }
}

impl Default for FocusManager {
  fn default() -> Self {
    let mut arena = Arena::new();
    let root = arena.new_node(FocusNodeInfo {
      focus_type: FocusType::SCOPE,
      wid: None,
    });
    Self {
      focusing: None,
      node_ids: HashMap::<WidgetId, NodeId>::new(),
      arena,
      root,
    }
  }
}

bitflags! {
  pub(crate) struct FocusType: u8 {
    const SCOPE = 0x01;
    const NODE = 0x02;
  }
}

#[derive(Debug)]
pub(crate) struct FocusNodeInfo {
  pub focus_type: FocusType,
  pub wid: Option<WidgetId>,
}

impl FocusManager {
  pub(crate) fn add_focus_node(
    &mut self,
    wid: WidgetId,
    auto_focus: bool,
    focus_type: FocusType,
    arena: &TreeArena,
  ) {
    if let Some(id) = self.node_ids.get(&wid) {
      let node = self.arena[*id].get_mut();
      assert!(node.wid == Some(wid) && !node.focus_type.intersects(focus_type));
      node.focus_type = node.focus_type.union(focus_type);
    } else {
      let node_id = self
        .arena
        .new_node(FocusNodeInfo { focus_type, wid: Some(wid) });
      self.node_ids.insert(wid, node_id);

      let it = wid.ancestors(arena).skip(1);
      let parent = it
        .filter_map(|id| self.node_ids.get(&id))
        .next()
        .unwrap_or(&self.root);
      self.insert_node(*parent, node_id, wid, arena);
    }

    if focus_type == FocusType::NODE && self.focusing.is_none() && auto_focus {
      self.focusing = Some(wid);
    }
  }

  pub(crate) fn focus_handle(this: &Rc<RefCell<Self>>, wid: WidgetId) -> FocustHandle {
    FocustHandle { mgr: this.clone(), wid }
  }

  pub(crate) fn remove_focus_node(&mut self, wid: WidgetId, focus_type: FocusType) {
    if Some(wid) == self.focusing && focus_type.intersects(FocusType::NODE) {
      self.focusing = None;
    }
    if let Some(id) = self.node_ids.get(&wid) {
      let node = self.arena[*id].get_mut();
      assert!(node.focus_type.intersects(focus_type));
      node.focus_type.remove(focus_type);
      if node.focus_type.is_empty() {
        id.remove(&mut self.arena);
      }
    }
  }

  pub fn focus_to(&mut self, wid: Option<WidgetId>) { self.focusing = wid; }

  pub(crate) fn next_focus(&mut self, arena: &TreeArena) -> Option<WidgetId> {
    self.focus_move_circle(false, arena)
  }

  pub(crate) fn prev_focus(&mut self, arena: &TreeArena) -> Option<WidgetId> {
    self.focus_move_circle(true, arena)
  }

  fn focus_move_circle(&mut self, backward: bool, arena: &TreeArena) -> Option<WidgetId> {
    let has_focus = self.focusing.is_some();
    let mut wid = self.focus_step(self.focusing, backward, arena);
    if wid.is_none() && has_focus {
      wid = self.focus_step(wid, backward, arena);
    }
    self.focusing = wid;
    wid
  }

  fn focus_step(
    &mut self,
    focusing: Option<WidgetId>,
    backward: bool,
    arena: &TreeArena,
  ) -> Option<WidgetId> {
    let mut node_id = focusing.and_then(|id| self.node_ids.get(&id)).copied();
    let mut scope_id = node_id.and_then(|id| self.scope_id(id)).or(Some(self.root));
    loop {
      scope_id?;
      let next = self.focus_step_in_scope(scope_id.unwrap(), node_id, backward, arena);
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
    arena: &TreeArena,
  ) -> Vec<(i16, NodeId, FocusType)> {
    let scope_tab_type = |id| {
      let mut v = vec![];
      let node = self.focus_scope_node(id, arena);
      if node.can_focus {
        v.push(FocusType::NODE);
      }
      if !node.skip_descendants {
        v.push(FocusType::SCOPE);
      }
      v
    };
    let is_scope = |id| self.assert_get(id).focus_type.intersects(FocusType::SCOPE);
    let node_type = |id| {
      self
        .arena
        .get(id)
        .map(|n| n.get())
        .map_or(vec![FocusType::NODE], |node| {
          if node.focus_type.intersects(FocusType::SCOPE) {
            scope_tab_type(node.wid)
          } else {
            vec![FocusType::NODE]
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
      let tab_index = self.tab_index(id, arena);
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
    arena: &TreeArena,
  ) -> Option<NodeId> {
    let mut iter = self
      .collect_tab_index_in_scope(scope_id, backward, arena)
      .into_iter();
    let mut tmp;
    let it: &mut dyn Iterator<Item = (i16, NodeId, FocusType)> = if let Some(node_id) = node_id {
      tmp = iter.skip_while(move |(_, id, _)| *id != node_id).skip(1);
      &mut tmp
    } else {
      &mut iter
    };

    for (_, id, focus_type) in it {
      let next = if focus_type == FocusType::SCOPE {
        self.focus_step_in_scope(id, None, backward, arena)
      } else {
        Some(id)
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
      .take_while(|n| self.assert_get(*n).focus_type.intersects(FocusType::SCOPE))
  }

  fn ignore_scope_id(&self, wid: WidgetId, arena: &TreeArena) -> Option<NodeId> {
    let node_id = wid
      .ancestors(arena)
      .filter_map(|wid| self.node_ids.get(&wid).copied())
      .next();
    node_id.and_then(|node_id| {
      self.scope_list(node_id).find(|id| {
        let mut has_ignore = false;
        self.get(*id).and_then(|n| n.wid).map(|wid| {
          wid.get(arena).map(|r| {
            r.query_on_first_type(QueryOrder::InnerFirst, |s: &FocusScope| {
              has_ignore = s.skip_descendants;
            })
          })
        });
        has_ignore
      })
    })
  }

  fn focus_scope_node(&self, scope_id: Option<WidgetId>, arena: &TreeArena) -> FocusScope {
    scope_id
      .and_then(|wid| {
        wid.get(arena).and_then(|r| {
          let mut node = None;
          r.query_on_first_type(QueryOrder::InnerFirst, |s: &FocusScope| {
            node = Some(s.clone());
          });
          node
        })
      })
      .unwrap_or_default()
  }

  fn tab_index(&self, node_id: NodeId, arena: &TreeArena) -> i16 {
    let mut index = 0;
    self
      .get(node_id)
      .and_then(|n| n.wid)
      .and_then(|wid| wid.get(arena))
      .map(|r| {
        r.query_on_first_type(QueryOrder::InnerFirst, |s: &FocusNode| {
          index = s.tab_index;
        });
      });

    index
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

impl Dispatcher {
  pub fn next_focus_widget(&mut self, tree: &WidgetTree) {
    self.focus_mgr.borrow_mut().next_focus(&tree.arena);
  }

  pub fn prev_focus_widget(&mut self, tree: &WidgetTree) {
    self.focus_mgr.borrow_mut().prev_focus(&tree.arena);
  }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(&mut self, tree: &mut WidgetTree) -> Option<WidgetId> {
    self.change_focusing_to(None, tree)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focus_mgr.borrow_mut().focusing }

  pub fn refresh_focus(&mut self, tree: &WidgetTree) {
    let focusing = self.focus_mgr.borrow().focusing.filter(|node_id| {
      self
        .focus_mgr
        .borrow()
        .ignore_scope_id(*node_id, &tree.arena)
        .is_none()
    });
    if self.focus_widgets.get(0) != focusing.as_ref() {
      self.change_focusing_to(focusing, tree);
    }
  }

  pub fn focus(&mut self, wid: WidgetId, tree: &WidgetTree) {
    self.change_focusing_to(Some(wid), tree);
  }

  fn change_focusing_to(&mut self, node: Option<WidgetId>, tree: &WidgetTree) -> Option<WidgetId> {
    let Self { focus_mgr, info, .. } = self;
    let old_widgets = &self.focus_widgets;
    let new_widgets = node.map_or(vec![], |wid| wid.ancestors(&tree.arena).collect::<Vec<_>>());

    let old = old_widgets
      .get(0)
      .filter(|wid| !(*wid).is_dropped(&tree.arena))
      .copied();

    // dispatch blur event
    if let Some(wid) = old {
      let mut focus_event = FocusEvent::new(wid, tree, info);
      wid
        .assert_get(&tree.arena)
        .query_on_first_type(QueryOrder::InnerFirst, |blur: &BlurListener| {
          blur.dispatch(&mut focus_event)
        })
    };

    let common_ancestors = common_ancestors(&new_widgets, old_widgets);
    // bubble focus out
    if let Some(wid) = old_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(&tree.arena))
    {
      let mut focus_event = FocusEvent::new(*wid, tree, info);
      tree.bubble_event_with(&mut focus_event, |focus_out: &FocusOutListener, event| {
        if common_ancestors.contains(&event.current_target()) {
          event.stop_bubbling();
        } else {
          focus_out.dispatch(event);
        }
      });
    };

    if let Some(wid) = node {
      let mut focus_event = FocusEvent::new(wid, tree, info);

      wid
        .assert_get(&tree.arena)
        .query_on_first_type(QueryOrder::InnerFirst, |focus: &FocusListener| {
          focus.dispatch(&mut focus_event)
        });

      let mut focus_event = FocusEvent::new(wid, tree, info);

      // bubble focus in
      tree.bubble_event_with(&mut focus_event, |focus_in: &FocusInListener, event| {
        if common_ancestors.contains(&event.current_target()) {
          event.stop_bubbling();
        } else {
          focus_in.dispatch(event);
        }
      });
    }

    self.focus_widgets = new_widgets;
    focus_mgr.borrow_mut().focusing = node;
    old
  }
}

fn common_ancestors(path: &[WidgetId], path2: &[WidgetId]) -> HashSet<WidgetId> {
  let it = path
    .iter()
    .rev()
    .zip(path2.iter().rev())
    .take_while(|(a, b)| a == b)
    .map(|(a, _)| a);
  let mut set = HashSet::new();
  set.extend(it);
  set
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::*;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  fn two_auto_focus() {
    // two auto focus widget
    let size = Size::zero();
    let widget = widget! {
      MockMulti  {
        MockBox { size, auto_focus: true, }
        MockBox { size, auto_focus: true, }
      }
    };

    let wnd = Window::default_mock(widget, None);
    let tree = &wnd.widget_tree;

    let id = tree.root().first_child(&tree.arena);
    assert!(id.is_some());
    assert_eq!(wnd.dispatcher.focusing(), id);
  }

  #[test]
  fn on_auto_focus() {
    // one auto focus widget
    let size = Size::zero();
    let widget = widget! {
      MockMulti {
        MockBox { size }
        MockBox { size, auto_focus: true}
      }
    };

    let wnd = Window::default_mock(widget, None);
    let tree = &wnd.widget_tree;

    let id = tree
      .root()
      .first_child(&tree.arena)
      .and_then(|p| p.next_sibling(&tree.arena));
    assert!(id.is_some());
    assert_eq!(wnd.dispatcher.focusing(), id);
  }

  #[test]
  fn tab_index() {
    let size = Size::zero();
    let widget = widget! {
      MockMulti {
        MockBox { size, tab_index: -1, }
        MockBox { size, tab_index: 0, }
        MockBox { size, tab_index: 1, auto_focus: true}
        MockBox { size, tab_index: 2, }
        MockMulti { tab_index: 4, MockBox { size, tab_index: 3, } }
        MockBox { size, tab_index: 0 }
      }
    };

    let mut wnd = Window::default_mock(widget.into_widget(), None);
    let Window { dispatcher, widget_tree, .. } = &mut wnd;
    dispatcher.refresh_focus(widget_tree);

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
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id2));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id3));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id4));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id0));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id0_0));
      dispatcher.next_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id1));

      // previous focus sequential

      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id0_0));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id0));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id4));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id3));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id2));
      dispatcher.prev_focus_widget(widget_tree);
      assert_eq!(dispatcher.focusing(), Some(id1));
    }
  }

  #[test]
  fn focus_event() {
    #[derive(Debug, Default)]
    struct EmbedFocus {
      log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Compose for EmbedFocus {
      fn compose(this: StateWidget<Self>) -> Widget {
        widget! {
          states { this: this.into_stateful() }
          MockBox {
            size: INFINITY_SIZE,
            focus: move |_| { this.log.borrow_mut().push("focus parent"); },
            blur: move |_| { this.log.borrow_mut().push("blur parent"); },
            focus_in: move |_| { this.log.borrow_mut().push("focusin parent"); },
            focus_out: move |_| { this.log.borrow_mut().push("focusout parent"); },
            MockBox {
              size: Size::zero(),
              focus: move |_| { this.log.borrow_mut().push("focus child"); },
              blur: move |_| { this.log.borrow_mut().push("blur child"); },
              focus_in: move |_| { this.log.borrow_mut().push("focusin child"); },
              focus_out: move |_| { this.log.borrow_mut().push("focusout child"); },
            }
          }
        }
      }
    }

    let widget = EmbedFocus::default();
    let log = widget.log.clone();
    let mut wnd = Window::default_mock(widget.into_widget(), None);
    let Window { dispatcher, widget_tree: tree, .. } = &mut wnd;

    let parent = tree.root();
    let child = parent.first_child(&tree.arena).unwrap();

    dispatcher.refresh_focus(tree);
    dispatcher.focus(child, tree);

    assert_eq!(
      &*log.borrow(),
      &["focus child", "focusin child", "focusin parent"]
    );
    log.borrow_mut().clear();

    dispatcher.focus(parent, tree);
    assert_eq!(
      &*log.borrow(),
      &["blur child", "focusout child", "focus parent",]
    );
    log.borrow_mut().clear();

    dispatcher.blur(tree);
    assert_eq!(&*log.borrow(), &["blur parent", "focusout parent",]);
  }
}
