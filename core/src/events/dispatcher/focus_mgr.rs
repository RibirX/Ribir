use crate::prelude::*;
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
  pub fn next_focus_widget(&mut self, ctx: &mut Context) -> Option<WidgetId> {
    let next = self
      .focusing
      .filter(|(_, index0)| *index0 < usize::MAX)
      .and_then(|(_, index0)| {
        let next = index0 + 1;
        self.tab_orders.get(next).map(|node| (*node, next))
      })
      .or_else(|| self.tab_orders.first().map(|node| (*node, 0)));

    self.change_focusing_to(next, ctx);
    self.focusing.map(|(node, _)| node.wid)
  }

  /// Switch to previous focus widget and return it.
  pub fn prev_focus_widget(&mut self, ctx: &mut Context) -> Option<WidgetId> {
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

    self.change_focusing_to(prev, ctx);
    self.focusing.map(|(node, _)| node.wid)
  }

  /// This method sets focus on the specified widget across its id `wid`.
  pub fn focus(&mut self, wid: WidgetId, ctx: &mut Context) {
    let node = self
      .tab_orders
      .iter()
      .enumerate()
      .find(|(_, node)| node.wid == wid)
      .map(|(idx, node)| (*node, idx));

    assert!(node.is_some());
    self.change_focusing_to(node, ctx);
  }

  /// Removes keyboard focus from the current focusing widget and return its id.
  pub fn blur(&mut self, ctx: &mut Context) -> Option<WidgetId> {
    self.change_focusing_to(None, ctx).map(|(node, _)| node.wid)
  }

  /// return the focusing widget.
  pub fn focusing(&self) -> Option<WidgetId> { self.focusing.map(|(node, _)| node.wid) }

  /// return the auto focus widget of the tree.
  pub fn auto_focus(&mut self, ctx: &Context) -> Option<WidgetId> {
    ctx.descendants().find(|id| {
      id.assert_get(&ctx.widget_tree)
        .as_attrs()
        .and_then(Attributes::find::<FocusAttr>)
        .map_or(false, |focus| focus.auto_focus)
    })
  }

  pub fn update(&mut self, ctx: &mut Context) {
    let tree = &ctx.widget_tree;
    self.tab_orders.clear();

    let mut zeros = vec![];
    tree
      .root()
      .descendants(tree)
      .filter_map(|id| {
        id.get(tree)
          .and_then(|w| w.as_attrs())
          .and_then(Attributes::find::<FocusAttr>)
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

    // if current focusing widget is dropped, find the next focus replace it.
    if let Some((focusing, _)) = self.focusing {
      if focusing.wid.is_dropped(tree) {
        // remove the dropped focusing.
        self.focusing = None;

        let node = self
          .tab_orders
          .iter()
          .enumerate()
          .find(|(_, node)| node.tab_index >= focusing.tab_index)
          .or_else(|| self.tab_orders.iter().enumerate().next())
          .map(|(idx, node)| (*node, idx));
        self.change_focusing_to(node, ctx);
      }
    }
  }

  fn change_focusing_to(
    &mut self,
    node: Option<(FocusNode, usize)>,
    ctx: &mut Context,
  ) -> Option<(FocusNode, usize)> {
    let old = self.focusing.take();
    self.focusing = node;

    if let Some((ref blur, _)) = old {
      // dispatch blur event
      if let Some(focus) = ctx.find_attr::<FocusAttr>(blur.wid) {
        let mut focus_event = FocusEvent::new(blur.wid, ctx);
        focus.dispatch_event(FocusEventType::Blur, &mut focus_event)
      }

      // bubble focus out
      ctx.bubble_event(
        blur.wid,
        |ctx, id| FocusEvent::new(id, ctx),
        |focus: &FocusAttr, event| focus.dispatch_event(FocusEventType::FocusOut, event),
      );
    }

    if let Some((focus, _)) = self.focusing {
      if let Some(focus_attr) = ctx.find_attr::<FocusAttr>(focus.wid) {
        let mut focus_event = FocusEvent::new(focus.wid, ctx);
        focus_attr.dispatch_event(FocusEventType::Focus, &mut focus_event)
      }

      // bubble focus out
      ctx.bubble_event(
        focus.wid,
        |ctx, id| FocusEvent::new(id, ctx),
        |focus: &FocusAttr, event| focus.dispatch_event(FocusEventType::FocusIn, event),
      );
    }

    old
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::SizedBox;
  use std::{cell::RefCell, ops::DerefMut, rc::Rc};

  fn empty_box() -> SizedBox { SizedBox { size: Size::zero() } }

  #[test]
  fn two_auto_focus() {
    // two auto focus widget
    let widget = Row::default()
      .have_child(empty_box().with_auto_focus(true).box_it())
      .have_child(empty_box().with_auto_focus(true).box_it());

    let ctx = Context::new(widget.box_it(), 1., None);
    let mut mgr = FocusManager::default();
    let tree = &ctx.widget_tree;

    let id = tree.root().first_child(tree);
    assert!(id.is_some());
    assert_eq!(mgr.auto_focus(&ctx), id);
  }

  #[test]
  fn on_auto_focus() {
    // one auto focus widget
    let widget = Row::default()
      .have_child(empty_box().box_it())
      .have_child(empty_box().with_auto_focus(true).box_it());

    let ctx = Context::new(widget.box_it(), 1., None);
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
    let widget = Row::default()
      .have_child(empty_box().with_tab_index(-1).box_it())
      .have_child(empty_box().with_tab_index(0).with_auto_focus(true).box_it())
      .have_child(empty_box().with_tab_index(1).box_it())
      .have_child(empty_box().with_tab_index(2).box_it())
      .have_child(empty_box().with_tab_index(3).box_it());

    let mut ctx = Context::new(widget.box_it(), 1., None);
    let mut mgr = FocusManager::default();
    mgr.update(&mut ctx);
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
      log: Rc<RefCell<Vec<String>>>,
    }

    impl CombinationWidget for EmbedFocus {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        let child = log_focus_event("child", empty_box(), self.log.clone());
        log_focus_event(
          "parent",
          SizedBox { size: SizedBox::expanded_size() },
          self.log.clone(),
        )
        .have_child(child.box_it())
        .box_it()
      }
    }

    fn log_focus_event<A: AttachAttr>(
      name: &'static str,
      widget: A,
      log: Rc<RefCell<Vec<String>>>,
    ) -> A::Target
    where
      A::Target: AttachAttr<Target = A::Target>,
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
    let mut ctx = Context::new(widget.box_it(), 1., None);
    let mut mgr = FocusManager::default();
    let tree = &ctx.widget_tree;

    let parent = tree.root().first_child(&tree).unwrap();
    let child = parent.first_child(&tree).unwrap();
    mgr.update(&mut ctx);
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

  #[test]
  fn fix_dropped_focusing() {
    struct T;

    impl CombinationWidget for T {
      #[widget]
      fn build(&self, ctx: &mut BuildCtx) -> BoxedWidget {
        widget! {
          declare SizedBox {
            size: Size::zero(),
            auto_focus: true,
          }
        }
      }
    }

    let w = T.into_stateful();
    let mut state_ref = unsafe { w.state_ref() };

    let mut wnd = Window::without_render(w.box_it(), Size::new(10., 10.));
    wnd.render_ready();

    // let child drop
    let _ = state_ref.deref_mut();

    wnd.render_ready();
  }
}
