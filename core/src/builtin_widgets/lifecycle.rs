use crate::{impl_all_event, impl_compose_child_for_listener, prelude::*, window::WindowId};
use rxrust::prelude::*;
use std::{convert::Infallible, rc::Rc};

define_widget_context!(LifecycleEvent);

pub type LifecycleSubject = MutRefItemSubject<'static, AllLifecycle, Infallible>;

#[derive(Default, Query)]
pub struct LifecycleListener {
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
    }) as fn(&mut AllLifecycle) -> Option<&mut LifecycleEvent>
  };
}

impl Declare for LifecycleListener {
  type Builder = Self;
  fn declare_builder() -> Self::Builder { Self::default() }
}

impl DeclareBuilder for LifecycleListener {
  type Target = State<Self>;
  fn build_declare(self, _: &BuildCtx) -> Self::Target { State::value(self) }
}

impl LifecycleListener {
  pub fn on_mounted(mut self, handler: impl FnMut(&mut LifecycleEvent) + 'static) -> Self {
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

  fn subject(&mut self) -> LifecycleSubject { self.lifecycle.clone() }
}

impl EventListener for LifecycleListener {
  type Event = AllLifecycle;
  #[inline]
  fn dispatch(&self, event: &mut Self::Event) { self.lifecycle.clone().next(event) }
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use crate::{
    prelude::*,
    reset_test_env,
    test_helper::{MockBox, MockMulti, TestWindow},
  };

  #[test]
  fn full_lifecycle() {
    reset_test_env!();

    let trigger = Stateful::new(true);
    let lifecycle = Stateful::new(vec![]);
    let c_lc = lifecycle.clone_reader();
    let c_trigger = trigger.clone_writer();

    let w = fn_widget! {
      @MockBox {
        size: Size::zero(),
        on_mounted: move |_| $lifecycle.write().push("static mounted"),
        on_performed_layout: move |_| $lifecycle.write().push("static performed layout"),
        on_disposed: move |_| $lifecycle.write().push("static disposed"),
        @ {
          pipe!(*$trigger).map(move  |b| {
            b.then(move || {
              @MockBox {
                size: Size::zero(),
                on_mounted: move |_| $lifecycle.write().push("dyn mounted"),
                on_performed_layout: move |_| $lifecycle.write().push("dyn performed layout"),
                on_disposed: move |_| $lifecycle.write().push("dyn disposed")
              }
            })
          })
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    assert_eq!(&**c_lc.read(), ["static mounted", "dyn mounted",]);

    wnd.draw_frame();

    assert_eq!(
      &**c_lc.read(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
      ]
    );
    {
      *c_trigger.write() = false;
    }
    wnd.draw_frame();
    assert_eq!(
      &**c_lc.read(),
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
    reset_test_env!();

    let cnt = Stateful::new(3);
    let mounted: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());
    let disposed: Stateful<HashSet<WidgetId>> = Stateful::new(HashSet::default());

    let c_cnt = cnt.clone_writer();
    let c_mounted = mounted.clone_reader();
    let c_disposed = disposed.clone_reader();
    let w = fn_widget! {
      @MockMulti {
        @ {
          pipe!(*$cnt).map(move |cnt| {
            (0..cnt).map(move |_| @MockBox {
              size: Size::zero(),
              on_mounted: move |e| { $mounted.write().insert(e.id); },
              on_disposed: move |e| { $disposed.write().insert(e.id); },
            })
          })
        }
      }
    };

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();
    let mounted_ids = c_mounted.read().clone();

    *c_cnt.write() = 5;
    wnd.draw_frame();

    assert_eq!(mounted_ids.len(), 3);
    assert_eq!(&mounted_ids, &*c_disposed.read());
  }
}
