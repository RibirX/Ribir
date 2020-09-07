use crate::{prelude::*, widget::widget_tree::WidgetTree};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct FocusManager {
  focus_order: BTreeMap<i16, Vec<WidgetId>>,
  focusing: Option<FocusNode>,
  auto_focus: std::collections::VecDeque<FocusNode>,
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
  pub fn drain_tab_index(&mut self, tab_index: i16, tree: &WidgetTree) {
    if let Some(current) = self.focusing {
      if current.wid.is_dropped(tree) {
        self.next_focus_widget(tree);
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
  pub fn next_focus_widget(&mut self, tree: &WidgetTree) -> Option<WidgetId> {
    self.focusing = if let Some(FocusNode { wid, tab_index }) = self.focusing {
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
    self.focusing.map(|node| node.wid)
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(&mut self, tree: &WidgetTree) -> Option<WidgetId> {
    self.focusing = if let Some(FocusNode { wid, tab_index }) = self.focusing {
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
    self.focusing.map(|node| node.wid)
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

    // next focus sequential
    assert_eq!(mgr.next_focus_widget(&tree), Some(id2));
    assert_eq!(mgr.next_focus_widget(&tree), Some(id22));
    assert_eq!(mgr.next_focus_widget(&tree), Some(id1));
    assert_eq!(mgr.next_focus_widget(&tree), Some(id0));
    assert_eq!(mgr.next_focus_widget(&tree), Some(id2));

    // previous focus sequential
    assert_eq!(mgr.prev_focus_widget(&tree), Some(id0));
    assert_eq!(mgr.prev_focus_widget(&tree), Some(id1));
    assert_eq!(mgr.prev_focus_widget(&tree), Some(id22));
    assert_eq!(mgr.prev_focus_widget(&tree), Some(id2));

    // drain filter
    id0.remove(&mut tree);
    mgr.drain_tab_index(0, &tree);
    assert_eq!(mgr.auto_focus(&tree), None);
    assert_eq!(mgr.focus_order.get(&0), None);
    assert_eq!(mgr.prev_focus_widget(&tree), Some(id1));
  }
}
