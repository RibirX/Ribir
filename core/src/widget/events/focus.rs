use crate::{
  prelude::*,
  widget::{dispatch::Dispatcher, widget_tree::WidgetTree, window::RawWindow},
};
use rxrust::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Default)]
pub struct FocusManager {
  /// store current focusing node, and its position in tab_orders.
  focusing: Option<(FocusNode, usize)>,
  tab_orders: Vec<FocusNode>,
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
  pub widget: BoxWidget,
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
  ///   by the tree's source order.
  /// - A positive value means the element should be focusable in sequential
  ///   keyboard navigation, with its order defined by the value of the number.
  ///   That is, tab_index=4 is focused before tab_index=5 and tab_index=0, but
  ///   after tab_index=3. If multiple elements share the same positive
  ///   tab_index value, their order relative to each other follows their
  ///   position in the tree source. The maximum value for tab_index is 32767.
  ///   If not specified, it takes the default value 0.
  pub tab_index: i16,
  /// Indicates whether the `widget` should automatically get focus when the
  /// window loads.
  ///
  /// Only one widget should have this attribute specified.  If there are
  /// several, the widget nearest the root, get the initial
  /// focus.
  pub auto_focus: bool,
  subject: LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()>,
}

#[derive(Debug, Clone, Copy)]
struct FocusNode {
  tab_index: i16,
  wid: WidgetId,
}

impl FocusManager {
  /// Switch to the next focus widget and return it.
  pub fn next_focus_widget(
    &mut self,
    tree: &mut WidgetTree,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Option<WidgetId> {
    let next = self
      .focusing
      .filter(|(_, index0)| *index0 < usize::MAX)
      .and_then(|(_, index0)| {
        let next = index0 + 1;
        self.tab_orders.get(next).map(|node| (*node, next))
      })
      .or_else(|| self.tab_orders.first().map(|node| (*node, 0)));

    self.change_focusing_to(next, modifiers, window, tree);
    self.focusing.map(|(node, _)| node.wid)
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(
    &mut self,
    tree: &mut WidgetTree,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Option<WidgetId> {
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

    self.change_focusing_to(prev, modifiers, window, tree);
    self.focusing.map(|(node, _)| node.wid)
  }

  /// This method sets focus on the specified widget across its id `wid`.
  pub fn focus(
    &mut self,
    wid: WidgetId,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
    tree: &mut WidgetTree,
  ) {
    let node = self
      .tab_orders
      .iter()
      .enumerate()
      .find(|(_, node)| node.wid == wid)
      .map(|(idx, node)| (*node, idx));

    assert!(node.is_some());
    self.change_focusing_to(node, modifiers, window, tree);
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
      .map(|(node, _)| node.wid)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focusing.map(|(node, _)| node.wid) }

  /// return the auto focus widget of the tree.
  pub fn auto_focus(&mut self, tree: &WidgetTree) -> Option<WidgetId> {
    tree.root().and_then(|root| {
      root.descendants(tree).find(|id| {
        id.get(tree)
          .and_then(|w| Widget::dynamic_cast_ref::<Focus>(w))
          .map_or(false, |focus| focus.auto_focus)
      })
    })
  }

  pub fn update(
    &mut self,
    tree: &mut WidgetTree,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) {
    self.tab_orders.clear();
    if let Some(root) = tree.root() {
      let mut zeros = vec![];
      root
        .descendants(tree)
        .filter_map(|id| {
          id.get(tree)
            .and_then(|w| Widget::dynamic_cast_ref::<Focus>(w))
            .map(|focus| FocusNode {
              tab_index: focus.tab_index,
              wid: id,
            })
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
        self.change_focusing_to(node, modifiers, window, tree);
      }
    }
  }

  fn change_focusing_to(
    &mut self,
    node: Option<(FocusNode, usize)>,
    modifiers: ModifiersState,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
    tree: &mut WidgetTree,
  ) -> Option<(FocusNode, usize)> {
    let old = std::mem::replace(&mut self.focusing, node);
    self.focusing = node;

    if let Some((ref blur, _)) = old {
      // dispatch blur event
      let event = Self::focus_event(blur.wid, modifiers, window.clone());
      Self::dispatch_event(blur.wid, tree, FocusEventType::Blur, event);

      // bubble focus out
      let event = Self::focus_event(blur.wid, modifiers, window.clone());
      Self::bubble_dispatch(blur.wid, FocusEventType::FocusOut, tree, event);
    }

    if let Some((focus, _)) = self.focusing {
      let event = Self::focus_event(focus.wid, modifiers, window.clone());
      Self::dispatch_event(focus.wid, tree, FocusEventType::Focus, event);

      // bubble focus out
      let event = Self::focus_event(focus.wid, modifiers, window);
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

  #[inline]
  pub fn focus_event_observable(
    &self,
  ) -> LocalSubject<'static, (FocusEventType, Rc<FocusEvent>), ()> {
    self.subject.clone()
  }

  pub fn listen_on<H: FnMut(&FocusEvent) + 'static>(
    base: BoxWidget,
    event_type: FocusEventType,
    mut handler: H,
  ) -> BoxWidget {
    let mut pointer = Self::from_widget(base, None, None);
    Widget::dynamic_cast_mut::<Self>(&mut pointer)
      .unwrap()
      .focus_event_observable()
      .filter(move |(t, _)| *t == event_type)
      .subscribe(move |(_, event)| handler(&*event));
    pointer
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{render::render_tree::RenderTree, widget::SizedBox};
  use widget::BoxWidget;

  fn empty_box() -> SizedBox { SizedBox::empty_box(Size::zero()) }

  fn mock_wnd() -> Rc<RefCell<Box<dyn RawWindow>>> {
    Rc::new(RefCell::new(Box::new(window::MockRawWindow::default())))
  }

  fn env<W: Widget>(widget: W) -> (WidgetTree, FocusManager) {
    let mut tree = WidgetTree::default();
    let mut r_tree = RenderTree::default();
    let mut mgr = FocusManager::default();
    tree.set_root(widget.box_it(), &mut r_tree);
    mgr.update(&mut tree, ModifiersState::default(), mock_wnd());
    (tree, mgr)
  }

  #[test]
  fn auto_focus() {
    // two auto focus widget
    let widget = Row::default()
      .push(empty_box().with_auto_focus(true))
      .push(empty_box().with_auto_focus(true));
    let (tree, mut mgr) = env(widget);
    let id = tree.root().and_then(|root| root.first_child(&tree));
    assert!(id.is_some());
    assert_eq!(mgr.auto_focus(&tree), id);

    // one auto focus widget
    let widget = Row::default()
      .push(empty_box())
      .push(empty_box().with_auto_focus(true));
    let (tree, mut mgr) = env(widget);
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
      .push(empty_box().with_tab_index(-1))
      .push(empty_box().with_tab_index(0).with_auto_focus(true))
      .push(empty_box().with_tab_index(1))
      .push(empty_box().with_tab_index(2))
      .push(empty_box().with_tab_index(3));

    let wnd = mock_wnd();
    let (mut tree, mut mgr) = env(widget);

    let negative = tree.root().unwrap().first_child(&tree).unwrap();
    let id0 = negative.next_sibling(&tree).unwrap();
    let id1 = id0.next_sibling(&tree).unwrap();
    let id2 = id1.next_sibling(&tree).unwrap();
    let id22 = id2.next_sibling(&tree).unwrap();

    {
      let mut next_focus = || mgr.next_focus_widget(&mut tree, <_>::default(), wnd.clone());
      // next focus sequential
      assert_eq!(next_focus(), Some(id1));
      assert_eq!(next_focus(), Some(id2));
      assert_eq!(next_focus(), Some(id22));
      assert_eq!(next_focus(), Some(id0));
      assert_eq!(next_focus(), Some(id1));

      // previous focus sequential
      let mut prev_focus = || mgr.prev_focus_widget(&mut tree, <_>::default(), wnd.clone());
      assert_eq!(prev_focus(), Some(id0));
      assert_eq!(prev_focus(), Some(id22));
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
      fn build(&self, _: &mut BuildCtx) -> BoxWidget {
        let child = log_focus_event("child", empty_box(), self.log.clone());
        log_focus_event("parent", SizedBox::expanded(child), self.log.clone())
      }
    }

    fn log_focus_event(
      name: &'static str,
      widget: impl Widget,
      log: Rc<RefCell<Vec<String>>>,
    ) -> BoxWidget {
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

    let wnd: Rc<RefCell<Box<dyn RawWindow>>> =
      Rc::new(RefCell::new(Box::new(window::MockRawWindow::default())));

    let widget = EmbedFocus::default();
    let log = widget.log.clone();
    let (mut tree, mut mgr) = env(widget);
    let parent = tree.root().unwrap().first_child(&tree).unwrap();
    let child = parent.first_child(&tree).unwrap();

    mgr.focus(child, ModifiersState::default(), wnd.clone(), &mut tree);
    assert_eq!(
      &*log.borrow(),
      &["focus child", "focusin child", "focusin parent"]
    );
    log.borrow_mut().clear();

    mgr.focus(parent, ModifiersState::default(), wnd.clone(), &mut tree);
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

    mgr.blur(ModifiersState::default(), wnd, &mut tree);
    assert_eq!(&*log.borrow(), &["blur parent", "focusout parent",]);
  }
}
