use crate::prelude::{widget_tree::WidgetTree, *};

use super::Dispatcher;
#[derive(Debug, Default)]
pub(crate) struct FocusManager {
  /// store current focusing node, and its position in tab_orders.
  focusing: Option<(FocusNode, usize)>,
  tab_orders: Vec<FocusNode>,
}

#[derive(Debug, Clone, Copy)]
struct FocusNode {
  tab_index: i16,
  wid: WidgetId,
}

impl Dispatcher {
  /// Switch to the next focus widget and return it.
  pub fn next_focus_widget(&mut self, tree: &mut WidgetTree) {
    let focus_mgr = &mut self.focus_mgr;
    let next = focus_mgr
      .focusing
      .filter(|(_, index0)| *index0 < usize::MAX)
      .and_then(|(_, index0)| {
        let next = index0 + 1;
        focus_mgr.tab_orders.get(next).map(|node| (*node, next))
      })
      .or_else(|| focus_mgr.tab_orders.first().map(|node| (*node, 0)));

    self.change_focusing_to(next, tree);
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(&mut self, tree: &mut WidgetTree) {
    let focus_mgr = &mut self.focus_mgr;
    let prev = focus_mgr
      .focusing
      .filter(|(_, index0)| *index0 > 0)
      .and_then(|(_, index0)| {
        let prev = index0 - 1;
        focus_mgr.tab_orders.get(prev).map(|node| (*node, prev))
      })
      .or_else(|| {
        focus_mgr
          .tab_orders
          .last()
          .map(|node| (*node, focus_mgr.tab_orders.len() - 1))
      });

    self.change_focusing_to(prev, tree);
  }

  /// This method sets focus on the specified widget across its id `wid`.
  pub fn focus(&mut self, wid: WidgetId, tree: &mut WidgetTree) {
    let focus_mgr = &mut self.focus_mgr;
    let node = focus_mgr
      .tab_orders
      .iter()
      .enumerate()
      .find(|(_, node)| node.wid == wid)
      .map(|(idx, node)| (*node, idx));

    assert!(node.is_some());
    self.change_focusing_to(node, tree);
  }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(&mut self, tree: &mut WidgetTree) -> Option<WidgetId> {
    self
      .change_focusing_to(None, tree)
      .map(|(node, _)| node.wid)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focus_mgr.focusing.map(|(node, _)| node.wid) }

  /// return the auto focus widget of the tree.
  pub fn auto_focus(&mut self, tree: &WidgetTree) -> Option<WidgetId> {
    tree.root().descendants(tree).find(|id| {
      let mut auto_focus = false;
      id.assert_get(tree)
        .query_on_first_type(QueryOrder::OutsideFirst, |focus: &FocusListener| {
          auto_focus = focus.auto_focus;
        });
      auto_focus
    })
  }

  pub fn refresh_focus(&mut self, tree: &mut WidgetTree) {
    let focus_mgr = &mut self.focus_mgr;
    focus_mgr.tab_orders.clear();

    let mut zeros = vec![];
    tree
      .root()
      .descendants(tree)
      .filter_map(|id| {
        let mut node = None;
        id.get(tree).map(|w| {
          w.query_on_first_type(QueryOrder::OutsideFirst, |focus: &FocusListener| {
            node = Some(FocusNode { tab_index: focus.tab_index, wid: id });
          })
        });
        node
      })
      .for_each(|node| match node.tab_index {
        0 => zeros.push(node),
        i if i > 0 => {
          focus_mgr.tab_orders.push(node);
          focus_mgr.tab_orders.sort_by_key(|node| node.tab_index);
        }
        _ => {}
      });
    focus_mgr.tab_orders.append(&mut zeros);

    // if current focusing widget is dropped, find the next focus replace it.
    if let Some((focusing, _)) = focus_mgr.focusing {
      if focusing.wid.is_dropped(tree) {
        // remove the dropped focusing.
        focus_mgr.focusing = None;

        let node = focus_mgr
          .tab_orders
          .iter()
          .enumerate()
          .find(|(_, node)| node.tab_index >= focusing.tab_index)
          .or_else(|| focus_mgr.tab_orders.iter().enumerate().next())
          .map(|(idx, node)| (*node, idx));
        self.change_focusing_to(node, tree);
      }
    }
  }

  fn change_focusing_to(
    &mut self,
    node: Option<(FocusNode, usize)>,
    tree: &mut WidgetTree,
  ) -> Option<(FocusNode, usize)> {
    let Self { focus_mgr, info, .. } = self;
    let old = focus_mgr.focusing.take();
    focus_mgr.focusing = node;

    if let Some((blur, _)) = old {
      let mut focus_event = FocusEvent::new(blur.wid, tree, info);
      // dispatch blur event
      blur
        .wid
        .assert_get_mut(tree)
        .query_on_first_type_mut(QueryOrder::InnerFirst, |focus: &mut FocusListener| {
          focus.dispatch_event(FocusEventType::Blur, &mut focus_event)
        });

      let mut focus_event = FocusEvent::new(blur.wid, tree, info);
      // bubble focus out
      tree.bubble_event_with(&mut focus_event, |focus: &mut FocusListener, event| {
        focus.dispatch_event(FocusEventType::FocusOut, event)
      });
    }

    if let Some((focus, _)) = focus_mgr.focusing {
      let mut focus_event = FocusEvent::new(focus.wid, tree, info);

      focus.wid.assert_get_mut(tree).query_on_first_type_mut(
        QueryOrder::InnerFirst,
        |focus_listener: &mut FocusListener| {
          focus_listener.dispatch_event(FocusEventType::Focus, &mut focus_event)
        },
      );

      let mut focus_event = FocusEvent::new(focus.wid, tree, info);

      // bubble focus in
      tree.bubble_event_with(&mut focus_event, |focus: &mut FocusListener, event| {
        focus.dispatch_event(FocusEventType::FocusIn, event)
      });
    }

    old
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::SizedBox;
  use std::{cell::RefCell, rc::Rc};

  #[test]
  fn two_auto_focus() {
    // two auto focus widget
    let size = Size::zero();
    let widget = widget! {
      Row  {
        SizedBox { size, auto_focus: true, }
        SizedBox { size, auto_focus: true, }
      }
    };

    let ctx = Context::new(widget.into_widget(), 1.);
    let mut mgr = FocusManager::default();
    let tree = &ctx.widget_tree;

    let id = tree.root().first_child(tree);
    assert!(id.is_some());
    assert_eq!(mgr.auto_focus(&ctx), id);
  }

  #[test]
  fn on_auto_focus() {
    // one auto focus widget
    let size = Size::zero();
    let widget = widget! {
      Row {
        SizedBox { size }
        SizedBox { size, auto_focus: true}
      }
    };

    let ctx = Context::new(widget.into_widget(), 1.);
    let mut mgr = FocusManager::default();
    let tree = &ctx.widget_tree;

    let id = tree
      .root()
      .first_child(tree)
      .and_then(|p| p.next_sibling(&tree));
    assert!(id.is_some());
    assert_eq!(mgr.auto_focus(&ctx), id);
  }

  #[test]
  fn tab_index() {
    let size = Size::zero();
    let widget = widget! {
      Row {
        SizedBox { size, tab_index: -1, }
        SizedBox { size, tab_index: 0, auto_focus: true }
        SizedBox { size, tab_index: 1, }
        SizedBox { size, tab_index: 2, }
        SizedBox { size, tab_index: 3, }
      }
    };

    let mut ctx = Context::new(widget.into_widget(), 1.);
    let mut mgr = FocusManager::default();
    mgr.refresh_focus(&mut ctx);
    let tree = &ctx.widget_tree;

    let negative = tree.root().first_child(&tree).unwrap();
    let id0 = negative.next_sibling(&tree).unwrap();
    let id1 = id0.next_sibling(&tree).unwrap();
    let id2 = id1.next_sibling(&tree).unwrap();
    let id3 = id2.next_sibling(&tree).unwrap();

    {
      // next focus sequential
      assert_eq!(mgr.next_focus_widget(&mut ctx), Some(id1));
      assert_eq!(mgr.next_focus_widget(&mut ctx), Some(id2));
      assert_eq!(mgr.next_focus_widget(&mut ctx), Some(id3));
      assert_eq!(mgr.next_focus_widget(&mut ctx), Some(id0));
      assert_eq!(mgr.next_focus_widget(&mut ctx), Some(id1));

      // previous focus sequential
      assert_eq!(mgr.prev_focus_widget(&mut ctx), Some(id0));
      assert_eq!(mgr.prev_focus_widget(&mut ctx), Some(id3));
      assert_eq!(mgr.prev_focus_widget(&mut ctx), Some(id2));
      assert_eq!(mgr.prev_focus_widget(&mut ctx), Some(id1));
    }
  }

  #[test]
  fn focus_event() {
    #[derive(Debug, Default)]
    struct EmbedFocus {
      log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Compose for EmbedFocus {
      fn compose(this: Stateful<Self>, _: &mut BuildCtx) -> Widget {
        widget! {
          track  { this }
          SizedBox {
            size: INFINITY_SIZE,
            on_focus: move |_| { this.log.borrow_mut().push("focus parent"); },
            on_blur: move |_| { this.log.borrow_mut().push("blur parent"); },
            on_focus_in: move |_| { this.log.borrow_mut().push("focusin parent"); },
            on_focus_out: move |_| { this.log.borrow_mut().push("focusout parent"); },
            SizedBox {
              size: Size::zero(),
              on_focus: move |_| { this.log.borrow_mut().push("focus child"); },
              on_blur: move |_| { this.log.borrow_mut().push("blur child"); },
              on_focus_in: move |_| { this.log.borrow_mut().push("focusin child"); },
              on_focus_out: move |_| { this.log.borrow_mut().push("focusout child"); },
            }
          }
        }
      }
    }

    let widget = EmbedFocus::default();
    let log = widget.log.clone();
    let mut ctx = Context::new(widget.into_widget(), 1.);
    let mut mgr = FocusManager::default();
    let tree = &ctx.widget_tree;

    let parent = tree.root();
    let child = parent
      .first_child(&tree)
      .unwrap()
      .first_child(&tree)
      .unwrap();
    mgr.refresh_focus(&mut ctx);
    mgr.focus(child, &mut ctx);

    assert_eq!(
      &*log.borrow(),
      &["focus child", "focusin child", "focusin parent"]
    );
    log.borrow_mut().clear();

    mgr.focus(parent, &mut ctx);
    assert_eq!(
      &*log.borrow(),
      &[
        "blur child",
        "focusout child",
        "focusout parent",
        "focus parent",
        "focusin parent"
      ]
    );
    log.borrow_mut().clear();

    mgr.blur(&mut ctx);
    assert_eq!(&*log.borrow(), &["blur parent", "focusout parent",]);
  }
}
