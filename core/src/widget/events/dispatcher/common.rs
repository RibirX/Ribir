use crate::{
  prelude::*,
  render::render_tree::RenderTree,
  widget::{
    widget_tree::{WidgetId, WidgetTree},
    window::RawWindow,
  },
};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};
pub(crate) struct CommonDispatcher {
  render_tree: NonNull<RenderTree>,
  widget_tree: NonNull<WidgetTree>,
  pub(crate) modifiers: ModifiersState,
  pub(crate) window: Rc<RefCell<Box<dyn RawWindow>>>,
}

impl CommonDispatcher {
  pub fn new(
    render_tree: NonNull<RenderTree>,
    widget_tree: NonNull<WidgetTree>,
    window: Rc<RefCell<Box<dyn RawWindow>>>,
  ) -> Self {
    Self {
      render_tree,
      widget_tree,
      modifiers: <_>::default(),
      window,
    }
  }

  #[inline]
  pub fn modifiers_change(&mut self, state: ModifiersState) { self.modifiers = state }

  #[inline]
  pub fn render_tree_ref(&self) -> &RenderTree { unsafe { self.render_tree.as_ref() } }

  #[inline]
  pub fn widget_tree_ref(&self) -> &WidgetTree { unsafe { self.widget_tree.as_ref() } }

  pub fn dispatch_to<
    Event: std::convert::AsMut<EventCommon> + std::fmt::Debug,
    Handler: FnMut(&WidgetAttr<BoxWidget, AttrData>, Rc<Event>),
    AttrData: std::fmt::Debug + 'static,
  >(
    &self,
    wid: WidgetId,
    handler: &mut Handler,
    event: Event,
  ) -> Event {
    let event_widget = wid
      .get(self.widget_tree_ref())
      .and_then(|w| w.downcast_attr_widget());
    if let Some(w) = event_widget {
      Self::rc_dispatch(w, event, handler)
    } else {
      event
    }
  }

  pub fn bubble_dispatch<
    Event: AsMut<EventCommon> + AsRef<EventCommon> + std::fmt::Debug,
    Handler: FnMut(&WidgetAttr<BoxWidget, AttrData>, Rc<Event>),
    EventDataUpdate: FnMut(&mut Event),
    AttrData: std::fmt::Debug + 'static,
  >(
    &self,
    wid: WidgetId,
    mut handler: Handler,
    event: Event,
    // Calling before dispatch event to the target widget, give an chance to update event data.
    mut update_event: EventDataUpdate,
  ) -> Event {
    let tree = self.widget_tree_ref();
    let res = wid
      .ancestors(tree)
      .filter_map(|wid| {
        wid
          .get(tree)
          .and_then(|w| w.downcast_attr_widget())
          .map(|widget| (wid, widget))
      })
      .try_fold(event, |mut event, (wid, widget)| {
        event.as_mut().current_target = wid;
        update_event(&mut event);
        event = Self::rc_dispatch(widget, event, &mut handler);
        Self::ok_bubble(event)
      });

    match res {
      Ok(event) => event,
      Err(event) => event,
    }
  }

  pub fn ok_bubble<Event: AsRef<EventCommon>>(e: Event) -> Result<Event, Event> {
    if e.as_ref().cancel_bubble.get() {
      Err(e)
    } else {
      Ok(e)
    }
  }

  fn rc_dispatch<W, Event, Handler>(widget: &W, event: Event, handler: &mut Handler) -> Event
  where
    W: Widget,
    Event: std::fmt::Debug,
    Handler: FnMut(&W, Rc<Event>),
  {
    let rc_event = Rc::new(event);
    handler(widget, rc_event.clone());
    Rc::try_unwrap(rc_event).expect("Keep the event is dangerous and not allowed")
  }
}
