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
    T: Widget,
    E: std::convert::AsMut<EventCommon> + std::fmt::Debug,
    H: FnMut(&T, Rc<E>),
  >(
    &self,
    wid: WidgetId,
    handler: &mut H,
    mut event: E,
  ) -> E {
    let event_widget = wid
      .get(self.widget_tree_ref())
      .and_then(|w| Widget::dynamic_cast_ref::<T>(w));
    if let Some(w) = event_widget {
      let common = event.as_mut();
      common.current_target = wid;
      common.composed_path.push(wid);

      let rc_event = Rc::new(event);
      handler(w, rc_event.clone());
      event = Rc::try_unwrap(rc_event).expect("Keep the event is dangerous and not allowed");
    }
    event
  }

  pub fn bubble_dispatch<
    T: Widget,
    E: AsMut<EventCommon> + AsRef<EventCommon> + std::fmt::Debug,
    H: FnMut(&T, Rc<E>),
  >(
    &self,
    wid: WidgetId,
    mut handler: H,
    event: E,
  ) {
    let _ = wid
      .ancestors(self.widget_tree_ref())
      .try_fold(event, |event, widget| {
        let e = self.dispatch_to(widget, &mut handler, event);
        Self::ok_bubble(e)
      });
  }

  pub fn ok_bubble<Event: AsRef<EventCommon>>(e: Event) -> Result<Event, Event> {
    if e.as_ref().cancel_bubble.get() {
      Err(e)
    } else {
      Ok(e)
    }
  }
}
