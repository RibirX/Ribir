use super::CommonDispatcher;
use crate::{prelude::*, widget::widget_tree::WidgetTree};
use rxrust::prelude::*;
use std::rc::Rc;
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

impl FocusManager {
  /// Switch to the next focus widget and return it.
  pub fn next_focus_widget(&mut self, dispatcher: &CommonDispatcher) -> Option<WidgetId> {
    let next = self
      .focusing
      .filter(|(_, index0)| *index0 < usize::MAX)
      .and_then(|(_, index0)| {
        let next = index0 + 1;
        self.tab_orders.get(next).map(|node| (*node, next))
      })
      .or_else(|| self.tab_orders.first().map(|node| (*node, 0)));

    self.change_focusing_to(next, dispatcher);
    self.focusing.map(|(node, _)| node.wid)
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(&mut self, dispatcher: &CommonDispatcher) -> Option<WidgetId> {
    let prev = self
      .focusing
      .filter(|(_, index0)| *index0 > 0)
      .and_then(|(_, index0)| {
        let prev = index0 - 1;
        self.tab_orders.get(prev).map(|node| (*node, prev))
      })
      .or_else(|| {
        self
          .tab_orders
          .last()
          .map(|node| (*node, self.tab_orders.len() - 1))
      });

    self.change_focusing_to(prev, dispatcher);
    self.focusing.map(|(node, _)| node.wid)
  }

  /// This method sets focus on the specified widget across its id `wid`.
  pub fn focus(&mut self, wid: WidgetId, dispatcher: &CommonDispatcher) {
    let node = self
      .tab_orders
      .iter()
      .enumerate()
      .find(|(_, node)| node.wid == wid)
      .map(|(idx, node)| (*node, idx));

    assert!(node.is_some());
    self.change_focusing_to(node, dispatcher);
  }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(&mut self, dispatcher: &CommonDispatcher) -> Option<WidgetId> {
    self
      .change_focusing_to(None, dispatcher)
      .map(|(node, _)| node.wid)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focusing.map(|(node, _)| node.wid) }

  /// return the auto focus widget of the tree.
  pub fn auto_focus(&mut self, tree: &WidgetTree) -> Option<WidgetId> {
    tree.root().and_then(|root| {
      root.descendants(tree).find(|id| {
        id.get(tree)
          .and_then(|w| (w as &dyn AttrsAccess).find_attr::<FocusAttr>())
          .map_or(false, |focus| focus.auto_focus)
      })
    })
  }

  pub fn update(&mut self, dispatcher: &CommonDispatcher) {
    let tree = dispatcher.widget_tree_ref();
    self.tab_orders.clear();
    if let Some(root) = tree.root() {
      let mut zeros = vec![];
      root
        .descendants(tree)
        .filter_map(|id| {
          id.get(tree)
            .and_then(|w| (w as &dyn AttrsAccess).find_attr::<FocusAttr>())
            .map(|focus| FocusNode { tab_index: focus.tab_index, wid: id })
        })
        .for_each(|node| match node.tab_index {
          0 => zeros.push(node),
          i if i > 0 => {
            self.tab_orders.push(node);
            self.tab_orders.sort_by_key(|node| node.tab_index);
          }
          _ => {}
        });
      self.tab_orders.append(&mut zeros);
    }

    // if current focusing widget is dropped, find the next focus replace it.
    if let Some((focusing, _)) = self.focusing {
      if focusing.wid.is_dropped(tree) {
        let node = self
          .tab_orders
          .iter()
          .enumerate()
          .find(|(_, node)| node.tab_index >= focusing.tab_index)
          .or_else(|| self.tab_orders.iter().enumerate().next())
          .map(|(idx, node)| (*node, idx));
        self.change_focusing_to(node, dispatcher);
      }
    }
  }

  fn change_focusing_to(
    &mut self,
    node: Option<(FocusNode, usize)>,
    dispatcher: &CommonDispatcher,
  ) -> Option<(FocusNode, usize)> {
    let old = std::mem::replace(&mut self.focusing, node);
    self.focusing = node;

    if let Some((ref blur, _)) = old {
      // dispatch blur event
      let event = Self::focus_event(blur.wid, dispatcher);
      dispatcher.dispatch_to(
        blur.wid,
        |f| FocusObserver::new(f, FocusEventType::Blur),
        event,
      );

      // bubble focus out
      let event = Self::focus_event(blur.wid, dispatcher);
      dispatcher.bubble_dispatch(
        blur.wid,
        |f| FocusObserver::new(f, FocusEventType::FocusOut),
        event,
        |_| {},
      );
    }

    if let Some((focus, _)) = self.focusing {
      let event = Self::focus_event(focus.wid, dispatcher);
      dispatcher.dispatch_to(
        focus.wid,
        |f| FocusObserver::new(f, FocusEventType::Focus),
        event,
      );

      // bubble focus out
      let event = Self::focus_event(focus.wid, dispatcher);
      dispatcher.bubble_dispatch(
        focus.wid,
        |f| FocusObserver::new(f, FocusEventType::FocusIn),
        event,
        |_| {},
      );
    }

    old
  }

  fn focus_event(wid: WidgetId, dispatcher: &CommonDispatcher) -> FocusEvent {
    FocusEvent::new(
      dispatcher.modifiers,
      wid,
      dispatcher.window.clone(),
      dispatcher.widget_tree,
      dispatcher.render_tree,
    )
  }
}

struct FocusObserver {
  event_type: FocusEventType,
  subject: LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()>,
}

impl FocusObserver {
  fn new(attr: &FocusAttr, event_type: FocusEventType) -> Self {
    Self {
      event_type,
      subject: attr.focus_event_observable(),
    }
  }
}

impl Observer for FocusObserver {
  type Item = Rc<FocusEvent>;
  type Err = ();

  fn next(&mut self, value: Self::Item) { self.subject.next((self.event_type, value)) }

  fn error(&mut self, err: Self::Err) { self.subject.error(err); }

  fn complete(&mut self) { self.subject.complete() }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::SizedBox;
  use std::cell::RefCell;

  fn empty_box() -> SizedBox { SizedBox::from_size(Size::zero()) }

  fn env(widget: BoxedWidget) -> (window::Window<window::MockRender>, FocusManager) {
    let wnd = window::NoRenderWindow::without_render(widget, Size::new(100., 100.));
    // use a aloneside FocusManager for test easy.
    let mut mgr = FocusManager::default();
    mgr.update(&wnd.dispatcher.common);
    (wnd, mgr)
  }

  #[test]
  fn auto_focus() {
    // two auto focus widget
    let widget = Row::default()
      .have(empty_box().with_auto_focus(true).box_it())
      .have(empty_box().with_auto_focus(true).box_it());
    let (wnd, mut mgr) = env(widget.box_it());
    let tree = wnd.dispatcher.common.widget_tree_ref();
    let id = tree.root().and_then(|root| root.first_child(&tree));
    assert!(id.is_some());
    assert_eq!(mgr.auto_focus(&tree), id);

    // one auto focus widget
    let widget = Row::default()
      .have(empty_box().box_it())
      .have(empty_box().with_auto_focus(true).box_it());
    let (wnd, mut mgr) = env(widget.box_it());
    let tree = wnd.dispatcher.common.widget_tree_ref();
    let id = tree
      .root()
      .and_then(|root| root.first_child(&tree))
      .and_then(|p| p.next_sibling(&tree));
    assert!(id.is_some());
    assert_eq!(mgr.auto_focus(&tree), id);
  }

  #[test]
  fn tab_index() {
    let widget = Row::default()
      .have(empty_box().with_tab_index(-1).box_it())
      .have(empty_box().with_tab_index(0).with_auto_focus(true).box_it())
      .have(empty_box().with_tab_index(1).box_it())
      .have(empty_box().with_tab_index(2).box_it())
      .have(empty_box().with_tab_index(3).box_it());

    let (wnd, mut mgr) = env(widget.box_it());
    let tree = wnd.dispatcher.common.widget_tree_ref();

    let negative = tree.root().unwrap().first_child(&tree).unwrap();
    let id0 = negative.next_sibling(&tree).unwrap();
    let id1 = id0.next_sibling(&tree).unwrap();
    let id2 = id1.next_sibling(&tree).unwrap();
    let id3 = id2.next_sibling(&tree).unwrap();

    {
      let mut next_focus = || mgr.next_focus_widget(&wnd.dispatcher.common);
      // next focus sequential
      assert_eq!(next_focus(), Some(id1));
      assert_eq!(next_focus(), Some(id2));
      assert_eq!(next_focus(), Some(id3));
      assert_eq!(next_focus(), Some(id0));
      assert_eq!(next_focus(), Some(id1));

      // previous focus sequential
      let mut prev_focus = || mgr.prev_focus_widget(&wnd.dispatcher.common);
      assert_eq!(prev_focus(), Some(id0));
      assert_eq!(prev_focus(), Some(id3));
      assert_eq!(prev_focus(), Some(id2));
      assert_eq!(prev_focus(), Some(id1));
    }
  }

  #[test]
  fn focus_event() {
    #[derive(Debug, Default)]
    struct EmbedFocus {
      log: Rc<RefCell<Vec<String>>>,
    }

    impl CombinationWidget for EmbedFocus {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        let child = log_focus_event("child", empty_box(), self.log.clone());
        log_focus_event("parent", SizedBox::expanded(), self.log.clone())
          .have(child.box_it())
          .box_it()
      }
    }

    fn log_focus_event<A: AttachAttr>(
      name: &'static str,
      widget: A,
      log: Rc<RefCell<Vec<String>>>,
    ) -> A::W
    where
      A::W: AttachAttr<W = A::W>,
    {
      let log2 = log.clone();
      let log3 = log.clone();
      let log4 = log.clone();
      widget
        .on_focus(move |_| {
          log.borrow_mut().push(format!("focus {}", name));
        })
        .on_blur(move |_| {
          log2.borrow_mut().push(format!("blur {}", name));
        })
        .on_focus_in(move |_| {
          log3.borrow_mut().push(format!("focusin {}", name));
        })
        .on_focus_out(move |_| {
          log4.borrow_mut().push(format!("focusout {}", name));
        })
    }

    let widget = EmbedFocus::default();
    let log = widget.log.clone();
    let (wnd, mut mgr) = env(widget.box_it());
    let tree = wnd.dispatcher.common.widget_tree_ref();
    let parent = tree.root().unwrap().first_child(&tree).unwrap();
    let child = parent.first_child(&tree).unwrap();

    mgr.focus(child, &wnd.dispatcher.common);
    assert_eq!(
      &*log.borrow(),
      &["focus child", "focusin child", "focusin parent"]
    );
    log.borrow_mut().clear();

    mgr.focus(parent, &wnd.dispatcher.common);
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

    mgr.blur(&wnd.dispatcher.common);
    assert_eq!(&*log.borrow(), &["blur parent", "focusout parent",]);
  }
}
