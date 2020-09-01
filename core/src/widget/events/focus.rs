use crate::prelude::*;
use std::collections::BTreeMap;

pub struct FocusManager {
  focus_order: BTreeMap<i16, Vec<WidgetId>>,
  focusing: Option<FocusNode>,
  auto_focus: Vec<FocusNode>,
}

impl FocusManager {
  pub fn add_new_focus_widget(&mut self, wid: WidgetId, widget: &Focus) {
    if widget.auto_focus {
      self.auto_focus.push(FocusNode {
        tab_index: widget.tab_index,
        wid,
      });
    }
    self
      .focus_order
      .entry(widget.tab_index)
      .or_insert_with(|| vec![])
      .push(wid);
  }

  /// Remove the destroyed widget which tab index is `tab_index`. Should not
  /// directly remove a focus widget, but use the tab index to batch remove.
  pub fn drain_tab_index(&mut self, tab_index: i16) {
    if let Some(current) = self.focusing {
      if current.wid.is_dropped() {
        self.next_focus_widget();
      }
    }

    // todo: focusing change
    let vec = self.focus_order.get_mut(&tab_index);
    if let Some(vec) = vec {
      vec.drain_filter(|w| w.is_dropped());
      if vec.is_empty() {
        self.focus_order.remove(&tab_index);
      }
    }
    self.auto_focus.drain_filter(|node| node.wid.is_dropped());
  }

  /// Switch to the next focus widget and return it.
  pub fn next_focus_widget(&mut self) -> Option<WidgetId> {
    self.focusing = if let Some(focus) = self.focusing {
      let iter = self
        .focus_iter(focus.tab_index..)
        .skip_while(|node| node.wid != focus.wid);
      Self::next_focus_node(iter)
    } else {
      let iter = self.focus_iter(..);
      Self::next_focus_node(iter)
    };
    self.focusing.map(|node| node.wid)
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(&mut self) -> Option<WidgetId> {
    self.focusing = if let Some(focus) = self.focusing {
      let iter = self
        .focus_iter(..=focus.tab_index)
        .rev()
        .skip_while(|node| node.wid != focus.wid);
      Self::next_focus_node(iter)
    } else {
      let iter = self.focus_iter(..).rev();
      Self::next_focus_node(iter)
    };
    self.focusing.map(|node| node.wid)
  }

  fn focus_iter<'a, R: std::ops::RangeBounds<i16>>(
    &'a self,
    rg: R,
  ) -> impl DoubleEndedIterator<Item = FocusNode> + 'a {
    self.focus_order.range(rg).flat_map(|(k, vec)| {
      vec.iter().map(move |wid| FocusNode {
        tab_index: *k,
        wid: *wid,
      })
    })
  }

  fn next_focus_node(iter: impl Iterator<Item = FocusNode>) -> Option<FocusNode> {
    iter.filter(|node| !node.wid.is_dropped()).next()
  }
}

/// Focus widget
#[derive(Debug)]
pub struct Focus {
  widget: BoxWidget,
  tab_index: i16,
  auto_focus: bool,
}

#[derive(Debug, Clone, Copy)]
struct FocusNode {
  tab_index: i16,
  wid: WidgetId,
}
