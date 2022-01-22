use crate::prelude::*;
use std::{any::Any, rc::Rc};

pub fn dispatch_to<E, O, F, Attr>(f: F, event: E) -> E
where
  E: std::convert::AsRef<EventCommon> + std::fmt::Debug,
  O: Observer<Item = Rc<E>, Err = ()>,
  F: Fn(&Attr) -> O,
  Attr: Any,
{
  let observer = event
    .context()
    .widget()
    .get_attrs()
    .and_then(Attributes::find)
    .map(|a| f(&*a));

  if let Some(o) = observer {
    rc_dispatch(event, o)
  } else {
    event
  }
}

pub fn bubble_dispatch<E, O, F, Attr, EventDataUpdate>(
  mut map_to_observer: F,
  event: E,
  // Calling before dispatch event to the target widget, give an chance to update event data.
  mut update_event: EventDataUpdate,
) -> E
where
  E: AsMut<EventCommon> + AsRef<EventCommon> + std::fmt::Debug,
  O: Observer<Item = Rc<E>, Err = ()>,
  F: Fn(&Attr) -> O,
  EventDataUpdate: FnMut(&mut E),
  Attr: Any,
{
  unimplemented!()
  // let (_, mut ancestors) = event.context().split_ancestors();
  // let res = ancestors.try_fold(event, |mut event, wid| {
  //   if let Some(observer) = event
  //     .context()
  //     .widget_by_id(wid)
  //     .get_attrs()
  //     .and_then(Attributes::find)
  //     .map(&mut map_to_observer)
  //   {
  //     event.as_mut().current_target = wid;
  //     update_event(&mut event);
  //     event = rc_dispatch(event, observer);
  //   }

  //   ok_bubble(event)
  // });

  // match res {
  //   Ok(event) => event,
  //   Err(event) => event,
  // }
}

pub fn ok_bubble<E: AsRef<EventCommon>>(e: E) -> Result<E, E> {
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
