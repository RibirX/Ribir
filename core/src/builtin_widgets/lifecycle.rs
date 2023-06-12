use std::convert::Infallible;

use crate::{impl_all_event, impl_compose_child_for_listener, impl_query_self_only, prelude::*};
use rxrust::{
  prelude::*,
  rc::{MutRc, RcDeref, RcDerefMut},
  subscriber::{Publisher, Subscriber},
};
use smallvec::SmallVec;

define_widget_context!(LifecycleEvent);

crate::events::impl_event_subject!(Lifecycle, event_name = AllLifecycle);

#[derive(Declare, Default)]
pub struct LifecycleListener {
  #[declare(skip)]
  lifecycle: LifecycleSubject,
}

impl_all_event!(
  Lifecycle,
  "Event fired when the widget is mounted. This event is fired only once.",
  Mounted,
  "Event fired when the widget is performed layout. This event may fire multiple \
   times in same frame if a widget modified after performed layout.",
  PerformedLayout,
  "Event fired when the widget is disposed. This event is fired only once.",
  Disposed
);

impl_compose_child_for_listener!(LifecycleListener);

impl LifecycleListener {
  #[inline]
  pub fn lifecycle_stream(&self) -> LifecycleSubject { self.lifecycle.clone() }
}

macro_rules! match_closure {
  ($event_ty: ident) => {
    (|e| match e {
      AllLifecycle::$event_ty(e) => Some(e),
      _ => None,
    }) as for<'a, 'b> fn(&'a mut AllLifecycle<'b>) -> Option<&'a mut LifecycleEvent<'b>>
  };
}

impl LifecycleListenerDeclarer {
  pub fn on_mounted(
    mut self,
    handler: impl for<'r> FnMut(&'r mut LifecycleEvent<'_>) + 'static,
  ) -> Self {
    let _ = self
      .subject()
      .filter_map(match_closure!(Mounted))
      .take(1)
      .subscribe(handler);

    self
  }

  pub fn on_performed_layout(mut self, handler: impl FnMut(&mut LifecycleEvent) + 'static) -> Self {
    let _ = self
      .subject()
      .filter_map(match_closure!(PerformedLayout))
      .subscribe(handler);

    self
  }

  pub fn on_disposed(mut self, handler: impl FnMut(&mut LifecycleEvent) + 'static) -> Self {
    let _ = self
      .subject()
      .filter_map(match_closure!(Disposed))
      .take(1)
      .subscribe(handler);

    self
  }

  fn subject(&mut self) -> LifecycleSubject {
    self
      .lifecycle
      .get_or_insert_with(LifecycleSubject::default)
      .clone()
  }
}

impl_query_self_only!(LifecycleListener);

impl EventListener for LifecycleListener {
  type Event<'a> = AllLifecycle<'a>;
  #[inline]
  fn dispatch(&self, event: &mut Self::Event<'_>) { self.lifecycle.clone().next(event) }
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use crate::{
    prelude::*,
    test_helper::{MockBox, MockMulti, TestWindow},
  };

  #[test]
  fn full_lifecycle() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let trigger = Stateful::new(true);
    let lifecycle = Stateful::new(vec![]);

    let w = widget! {
      states {
        trigger: trigger.clone(),
        lifecycle: lifecycle.clone()
      }
      MockBox {
        size: Size::zero(),
        on_mounted: move |_| lifecycle.silent().push("static mounted"),
        on_performed_layout: move |_| lifecycle.silent().push("static performed layout"),
        on_disposed: move |_| lifecycle.silent().push("static disposed"),
        widget::then(*trigger, || widget! {
          MockBox {
            size: Size::zero(),
            on_mounted: move |_| lifecycle.silent().push("dyn mounted"),
            on_performed_layout: move |_| lifecycle.silent().push("dyn performed layout"),
            on_disposed: move |_| lifecycle.silent().push("dyn disposed")
          }
        })
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    assert_eq!(&**lifecycle.state_ref(), ["static mounted"]);
    wnd.draw_frame();

    assert_eq!(
      &**lifecycle.state_ref(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
      ]
    );
    {
      *trigger.state_ref() = false;
    }
    wnd.draw_frame();
    assert_eq!(
      &**lifecycle.state_ref(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
        "static performed layout",
      ]
    );
  }

  #[test]
  fn track_lifecycle() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let cnt = Stateful::new(3);
    let mounted: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());
    let disposed: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());
    let w = widget! {
      states {
        cnt: cnt.clone(),
        mounted: mounted.clone(),
        disposed: disposed.clone(),
      }
      MockMulti {
        Multi::new((0..*cnt).map(move |_| widget! {
          MockBox {
            size: Size::zero(),
            on_mounted: move |ctx| {
              mounted.insert(ctx.id);
            },
            on_disposed: move |ctx| {
              disposed.insert(ctx.id);
            },
          }
        }))
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();
    let mounted_ids = (*mounted.state_ref()).clone();

    *cnt.state_ref() = 5;
    wnd.on_wnd_resize_event(Size::zero());
    wnd.draw_frame();

    let disposed_ids = (*disposed.state_ref()).clone();
    assert_eq!(mounted_ids.len(), 3);
    assert_eq!(mounted_ids, disposed_ids);
  }
}
