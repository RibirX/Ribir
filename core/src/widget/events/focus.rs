use crate::{
  prelude::*,
  widget::{dispatch::Dispatcher, widget_tree::WidgetTree, window::RawWindow},
};
use rxrust::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Debug, Default)]
pub struct FocusManager {
  focus_order: BTreeMap<i16, Vec<WidgetId>>,
  focusing: Option<FocusNode>,
  auto_focus: std::collections::VecDeque<FocusNode>,
}

pub type FocusEvent = EventCommon;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusEventType {
  /// The focus event fires when an widget has received focus. The main
  /// difference between this event and focusin is that focusin bubbles while
  /// focus does not.
  Focus,
  /// The blur event fires when an widget has lost focus. The main difference
  /// between this event and focusout is that focusout bubbles while blur does
  /// not.
  Blur,
  /// The focusin event fires when an widget is about to receive focus. The main
  /// difference between this event and focus is that focusin bubbles while
  /// focus does not.
  FocusIn,
  /// The focusout event fires when an widget is about to lose focus. The main
  /// difference between this event and blur is that focusout bubbles while blur
  /// does not.
  FocusOut,
}

/// Focus widget
#[derive(Debug)]
pub struct Focus {
  widget: BoxWidget,
  /// Indicates that `widget` can be focused, and where it participates in
  /// sequential keyboard navigation (usually with the Tab key, hence the name.
  ///
  /// It accepts an integer as a value, with different results depending on the
  /// integer's value:
  /// - A negative value (usually -1) means that the widget is not reachable via
  ///   sequential keyboard navigation, but could be focused with API or
  ///   visually by clicking with the mouse.
  /// - Zero means that the element should be focusable in sequential keyboard
  ///   navigation, after any positive tab_index values and its order is defined
  ///   by the document's source order.
  /// - A positive value means the element should be focusable in sequential
  ///   keyboard navigation, with its order defined by the value of the number.
  ///   That is, tab_index=4 is focused before tab_index=5 and tab_index=0, but
  ///   after tab_index=3. If multiple elements share the same positive
  ///   tab_index value, their order relative to each other follows their
  ///   position in the document source. The maximum value for tab_index is
  ///   32767. If not specified, it takes the default value 0.
  tab_index: i16,
  /// Indicates whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the first widget with the attribute set inserted, get the initial
  /// focus.
  auto_focus: bool,
  subject: LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()>,
}

#[derive(Debug, Clone, Copy)]
struct FocusNode {
  tab_index: i16,
  wid: WidgetId,
}

impl FocusManager {
  pub fn add_new_focus_widget(&mut self, wid: WidgetId, widget: &Focus) {
    if widget.auto_focus {
      self.auto_focus.push_back(FocusNode {
        tab_index: widget.tab_index,
        wid,
      });
    }
    self
      .focus_order
      .entry(widget.tab_index)
      .or_insert_with(Vec::new)
      .push(wid);
  }

  /// Remove the destroyed widget which tab index is `tab_index`. Should not
  /// directly remove a focus widget, but use the tab index to batch remove.
  pub fn drain_tab_index(
    &mut self,
    tab_index: i16,
    tree: &mut WidgetTree,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) {
    if let Some(current) = self.focusing {
      if current.wid.is_dropped(tree) {
        self.next_focus_widget(tree, modifiers, window);
      }
    }

    let vec = self.focus_order.get_mut(&tab_index);
    if let Some(vec) = vec {
      vec.drain_filter(|w| w.is_dropped(tree));
      if vec.is_empty() {
        self.focus_order.remove(&tab_index);
      }
    }
  }

  /// Switch to the next focus widget and return it.
  pub fn next_focus_widget(
    &mut self,
    tree: &mut WidgetTree,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Option<WidgetId> {
    let next = if let Some(FocusNode { wid, tab_index }) = self.focusing {
      // find the same tab_index widget next to current focusing.
      self
        .focus_order
        .get(&tab_index)
        .and_then(|vec| {
          vec
            .iter()
            .skip_while(|id| **id != wid)
            .skip(1)
            .find(|id| !id.is_dropped(tree))
            .map(|wid| FocusNode::new(tab_index, *wid))
        })
        // or get the nearest focus widget which `tab_index` less than current.
        .or_else(|| self.next_focus_in_range(0..tab_index, tree))
        // or enter the next cycle, get the largest `tab_index` focus widget.
        .or_else(|| self.next_focus_in_range(tab_index.., tree))
    } else {
      self.next_focus_in_range(0.., tree)
    };
    self.change_focusing_to(next, modifiers, window, tree);
    self.focusing.map(|node| node.wid)
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(
    &mut self,
    tree: &mut WidgetTree,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Option<WidgetId> {
    let prev = if let Some(FocusNode { wid, tab_index }) = self.focusing {
      // find the same tab_index widget next to current focusing.
      self
        .focus_order
        .get(&tab_index)
        .and_then(|vec| {
          vec
            .iter()
            .rev()
            .skip_while(|id| **id != wid)
            .skip(1)
            .find(|id| !id.is_dropped(tree))
            .map(|wid| FocusNode::new(tab_index, *wid))
        })
        // or get the nearest focus widget which `tab_index` greater than current.
        .or_else(|| self.prev_focus_in_range(tab_index + 1.., tree))
        // or enter the next cycle, get the least `tab_index` focus widget.
        .or_else(|| self.prev_focus_in_range(0..=tab_index, tree))
    } else {
      self.prev_focus_in_range(0.., tree)
    };
    self.change_focusing_to(prev, modifiers, window, tree);
    self.focusing.map(|node| node.wid)
  }

  /// This method sets focus on the specified widget across its id `wid`.
  pub fn focus(
    &mut self,
    wid: WidgetId,
    tab_index: i16,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
    tree: &mut WidgetTree,
  ) {
    debug_assert!(
      self
        .focus_order
        .get(&tab_index)
        .map_or(false, |vec| vec.contains(&wid))
    );
    self.change_focusing_to(Some(FocusNode { tab_index, wid }), modifiers, window, tree);
  }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(
    &mut self,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
    tree: &mut WidgetTree,
  ) -> Option<WidgetId> {
    self
      .change_focusing_to(None, modifiers, window, tree)
      .map(|wid| wid.wid)
  }

  pub fn auto_focus(&mut self, tree: &WidgetTree) -> Option<WidgetId> {
    while let Some(node) = self.auto_focus.front() {
      if node.wid.is_dropped(tree) {
        self.auto_focus.pop_front();
      } else {
        break;
      }
    }

    self.auto_focus.front().map(|node| node.wid)
  }

  fn next_focus_in_range<R: std::ops::RangeBounds<i16>>(
    &self,
    rg: R,
    tree: &WidgetTree,
  ) -> Option<FocusNode> {
    self
      .focus_order
      .range(rg)
      .rev()
      .find_map(|(tab_index, vec)| {
        vec
          .iter()
          .find(|wid| !wid.is_dropped(tree))
          .map(|wid| FocusNode::new(*tab_index, *wid))
      })
  }

  fn prev_focus_in_range<R: std::ops::RangeBounds<i16>>(
    &self,
    rg: R,
    tree: &WidgetTree,
  ) -> Option<FocusNode> {
    self.focus_order.range(rg).find_map(|(tab_index, vec)| {
      vec
        .iter()
        .rev()
        .find(|wid| !wid.is_dropped(tree))
        .map(|wid| FocusNode::new(*tab_index, *wid))
    })
  }

  fn change_focusing_to(
    &mut self,
    node: Option<FocusNode>,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
    tree: &mut WidgetTree,
  ) -> Option<FocusNode> {
    let old = std::mem::replace(&mut self.focusing, node);
    self.focusing = node;

    if let Some(ref blur) = old {
      let mut event = Self::focus_event(blur.wid, modifiers, window.clone());

      // dispatch blur event
      event = Self::dispatch_event(blur.wid, tree, FocusEventType::Blur, event);
      event.cancel_bubble.set(false);

      // bubble focus out
      Self::bubble_dispatch(blur.wid, FocusEventType::FocusOut, tree, event);
    }

    if let Some(focus) = self.focusing {
      let mut event = Self::focus_event(focus.wid, modifiers, window);
      // dispatch blur event
      event = Self::dispatch_event(focus.wid, tree, FocusEventType::Focus, event);
      event.cancel_bubble.set(false);

      // bubble focus out
      Self::bubble_dispatch(focus.wid, FocusEventType::FocusIn, tree, event);
    }

    old
  }

  fn focus_event(
    wid: WidgetId,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> FocusEvent {
    FocusEvent {
      target: wid,
      current_target: wid,
      composed_path: vec![],
      modifiers,
      cancel_bubble: <_>::default(),
      window,
    }
  }

  fn dispatch_event(
    wid: WidgetId,
    tree: &mut WidgetTree,
    event_type: FocusEventType,
    event: FocusEvent,
  ) -> FocusEvent {
    Dispatcher::dispatch_to_widget(
      wid,
      tree,
      &mut |focus: &mut Focus, event| {
        log::info!("{:?} {:?}", event_type, event);
        focus.subject.next((event_type, event));
      },
      event,
    )
  }

  fn bubble_dispatch(
    wid: WidgetId,
    event_type: FocusEventType,
    tree: &mut WidgetTree,
    event: FocusEvent,
  ) {
    let tree_ptr = tree as *mut WidgetTree;
    // Safety: we know below code will change the tree node, but never change the
    // tree.
    let (tree, tree2) = unsafe { (&mut *tree_ptr, &mut *tree_ptr) };
    let _ = wid.ancestors(tree).try_fold(event, |event, wid| {
      log::info!("{:?} {:?}", event_type, event);
      let event = Self::dispatch_event(wid, tree2, event_type, event);
      if event.cancel_bubble.get() {
        Err(())
      } else {
        Ok(event)
      }
    });
  }
}

inherit_widget!(Focus, widget);

impl Focus {
  pub fn from_widget(
    widget: BoxWidget,
    auto_focus: Option<bool>,
    tab_index: Option<i16>,
  ) -> BoxWidget {
    inherit(
      widget.box_it(),
      |base| Self {
        widget: base,
        tab_index: tab_index.unwrap_or(0),
        auto_focus: auto_focus.unwrap_or(false),
        subject: <_>::default(),
      },
      move |base| {
        if let Some(tab_index) = tab_index {
          base.tab_index = tab_index;
        }
        if let Some(auto_focus) = auto_focus {
          base.auto_focus = auto_focus;
        }
      },
    )
  }
}

impl FocusNode {
  fn new(tab_index: i16, wid: WidgetId) -> Self { FocusNode { tab_index, wid } }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::SizedBox;

  fn empty_box() -> SizedBox { SizedBox::empty_box(Size::zero()) }
  fn unwrap_focus(id: WidgetId, tree: &WidgetTree) -> &Focus {
    Widget::dynamic_cast_ref::<Focus>(id.get(tree).unwrap()).unwrap()
  }

  #[test]
  fn auto_focus() {
    let mut tree = WidgetTree::default();
    let mut mgr = FocusManager::default();

    let id0 = tree.new_node(empty_box().with_auto_focus(true));
    let id1 = tree.new_node(empty_box().with_auto_focus(true));
    mgr.add_new_focus_widget(id0, unwrap_focus(id0, &tree));
    mgr.add_new_focus_widget(id1, unwrap_focus(id1, &tree));

    assert_eq!(mgr.auto_focus(&tree), Some(id0));
    id0.remove(&mut tree);
    assert_eq!(mgr.auto_focus(&tree), Some(id1));
  }

  #[test]
  fn tab_index() {
    let wnd: Rc<RefCell<Box<dyn RawWindow>>> =
      Rc::new(RefCell::new(Box::new(window::MockRawWindow::default())));
    let mut tree = WidgetTree::default();
    let mut mgr = FocusManager::default();

    let negative = tree.new_node(empty_box().with_tab_index(-1));
    let id0 = tree.new_node(empty_box().with_tab_index(0).with_auto_focus(true));
    let id1 = tree.new_node(empty_box().with_tab_index(1));
    let id2 = tree.new_node(empty_box().with_tab_index(2));
    let id22 = tree.new_node(empty_box().with_tab_index(2));
    mgr.add_new_focus_widget(negative, unwrap_focus(negative, &tree));
    mgr.add_new_focus_widget(id0, unwrap_focus(id0, &tree));
    mgr.add_new_focus_widget(id1, unwrap_focus(id1, &tree));
    mgr.add_new_focus_widget(id2, unwrap_focus(id2, &tree));
    mgr.add_new_focus_widget(id22, unwrap_focus(id22, &tree));

    {
      let mut next_focus = || mgr.next_focus_widget(&mut tree, <_>::default(), wnd.clone());
      // next focus sequential
      assert_eq!(next_focus(), Some(id2));
      assert_eq!(next_focus(), Some(id22));
      assert_eq!(next_focus(), Some(id1));
      assert_eq!(next_focus(), Some(id0));
      assert_eq!(next_focus(), Some(id2));

      // previous focus sequential
      let mut prev_focus = || mgr.prev_focus_widget(&mut tree, <_>::default(), wnd.clone());
      assert_eq!(prev_focus(), Some(id0));
      assert_eq!(prev_focus(), Some(id1));
      assert_eq!(prev_focus(), Some(id22));
      assert_eq!(prev_focus(), Some(id2));
    }
    // drain filter
    id0.remove(&mut tree);
    mgr.drain_tab_index(0, &mut tree, <_>::default(), wnd.clone());
    assert_eq!(mgr.auto_focus(&tree), None);
    assert_eq!(mgr.focus_order.get(&0), None);
    assert_eq!(
      mgr.prev_focus_widget(&mut tree, <_>::default(), wnd),
      Some(id1)
    );
  }
}
