use crate::{
  prelude::*,
  render::render_tree::RenderTree,
  widget::{
    widget_tree::{WidgetId, WidgetTree},
    window::RawWindow,
  },
};
use std::{any::Any, cell::RefCell, ptr::NonNull, rc::Rc};
pub(crate) struct CommonDispatcher {
  pub(crate) render_tree: NonNull<RenderTree>,
  pub(crate) widget_tree: NonNull<WidgetTree>,
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
    O: Observer<Item = Rc<Event>, Err = ()>,
    F: Fn(&Attr) -> O,
    Attr: Any,
  >(
    &self,
    wid: WidgetId,
    f: F,
    event: Event,
  ) -> Event {
    let observer = wid
      .get(self.widget_tree_ref())
      .and_then(|w| w.get_attrs())
      .and_then(Attributes::find)
      .map(|a| f(&*a));
    if let Some(o) = observer {
      Self::rc_dispatch(event, o)
    } else {
      event
    }
  }

  pub fn bubble_dispatch<
    Event: AsMut<EventCommon> + AsRef<EventCommon> + std::fmt::Debug,
    O: Observer<Item = Rc<Event>, Err = ()>,
    F: Fn(&Attr) -> O,
    EventDataUpdate: FnMut(&mut Event),
    Attr: Any,
  >(
    &self,
    wid: WidgetId,
    map_to_observer: F,
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
          .and_then(|w| w.get_attrs())
          .and_then(Attributes::find)
          .map(|attr| (wid, map_to_observer(&*attr)))
      })
      .try_fold(event, |mut event, (wid, observer)| {
        event.as_mut().current_target = wid;
        update_event(&mut event);
        event = Self::rc_dispatch(event, observer);
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

  fn rc_dispatch<Event, O>(event: Event, mut observer: O) -> Event
  where
    Event: std::fmt::Debug,
    O: Observer<Item = Rc<Event>, Err = ()>,
  {
    let rc_event = Rc::new(event);
    observer.next(rc_event.clone());
    Rc::try_unwrap(rc_event).expect("Keep the event is dangerous and not allowed")
  }
}
