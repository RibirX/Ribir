use std::convert::Infallible;

use crate::{impl_query_self_only, prelude::*};
use rxrust::{
  prelude::*,
  rc::{MutRc, RcDeref, RcDerefMut},
  subscriber::{Publisher, Subscriber},
};
use smallvec::SmallVec;

use crate::context::LifeCycleCtx;

/// Listener perform when its child widget add to the widget tree.
#[derive(Declare)]
pub struct MountedListener {
  #[declare(builtin, convert=custom)]
  pub on_mounted: LifecycleSubject,
}

#[derive(Declare)]
pub struct PerformedLayoutListener {
  #[declare(builtin, convert=custom)]
  pub on_performed_layout: LifecycleSubject,
}

#[derive(Declare)]
pub struct DisposedListener {
  #[declare(builtin, convert=custom)]
  pub on_disposed: LifecycleSubject,
}

type LifecyclePublisher =
  MutRc<Option<SmallVec<[Box<dyn for<'r> Publisher<LifeCycleCtx<'r>, Infallible>>; 1]>>>;

#[derive(Clone)]
pub struct LifecycleSubject {
  observers: LifecyclePublisher,
  chamber: LifecyclePublisher,
}

impl<'a, O> Observable<LifeCycleCtx<'a>, Infallible, O> for LifecycleSubject
where
  O: for<'r> Observer<LifeCycleCtx<'r>, Infallible> + 'static,
{
  type Unsub = Subscriber<O>;

  fn actual_subscribe(self, observer: O) -> Self::Unsub {
    if let Some(chamber) = self.chamber.rc_deref_mut().as_mut() {
      self
        .observers
        .rc_deref_mut()
        .as_mut()
        .unwrap()
        .retain(|p| !p.p_is_closed());

      let subscriber = Subscriber::new(Some(observer));
      chamber.push(Box::new(subscriber.clone()));
      subscriber
    } else {
      Subscriber::new(None)
    }
  }
}
impl<'a> ObservableExt<LifeCycleCtx<'a>, ()> for LifecycleSubject {}

impl<'b> Observer<LifeCycleCtx<'b>, Infallible> for LifecycleSubject {
  fn next(&mut self, value: LifeCycleCtx<'b>) {
    self.load();
    if let Some(observers) = self.observers.rc_deref_mut().as_mut() {
      for p in observers.iter_mut() {
        p.p_next(value.clone());
      }
    }
  }

  fn error(self, _: Infallible) {}

  fn complete(mut self) {
    self.load();
    if let Some(observers) = self.observers.rc_deref_mut().take() {
      observers
        .into_iter()
        .filter(|o| !o.p_is_closed())
        .for_each(|subscriber| subscriber.p_complete());
    }
  }

  #[inline]
  fn is_finished(&self) -> bool { self.observers.rc_deref().is_none() }
}

impl LifecycleSubject {
  fn load(&mut self) {
    if let Some(observers) = self.observers.rc_deref_mut().as_mut() {
      observers.append(self.chamber.rc_deref_mut().as_mut().unwrap());
    }
  }
}

#[macro_export]
macro_rules! impl_lifecycle {
  ($listener: ident, $declarer:ident, $field: ident, $stream_name: ident) => {
    impl ComposeChild for $listener {
      type Child = Widget;
      fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
        compose_child_as_data_widget(child, this)
      }
    }

    impl $listener {
      #[inline]
      pub fn $stream_name(&self) -> LifecycleSubject { self.$field.clone() }

      #[inline]
      pub(crate) fn dispatch(&self, event: LifeCycleCtx<'_>) { self.$field.clone().next(event) }
    }

    impl $declarer {
      pub fn $field(mut self, handler: impl for<'r> FnMut(LifeCycleCtx<'r>) + 'static) -> Self {
        assert!(self.$field.is_none());
        let subject = LifecycleSubject::default();
        subject.clone().subscribe(handler);
        self.$field = Some(subject);
        self
      }
    }

    impl Query for $listener {
      impl_query_self_only!();
    }
  };
}

impl_lifecycle!(
  MountedListener,
  MountedListenerDeclarer,
  on_mounted,
  mounted_stream
);

impl_lifecycle!(
  PerformedLayoutListener,
  PerformedLayoutListenerDeclarer,
  on_performed_layout,
  performed_layout_stream
);

impl_lifecycle!(
  DisposedListener,
  DisposedListenerDeclarer,
  on_disposed,
  disposed_stream
);

impl<'a> Clone for LifeCycleCtx<'a> {
  fn clone(&self) -> Self {
    Self {
      id: self.id,
      arena: self.arena,
      store: self.store,
      wnd_ctx: self.wnd_ctx,
    }
  }
}

impl Default for LifecycleSubject {
  fn default() -> Self {
    Self {
      observers: MutRc::own(Some(<_>::default())),
      chamber: MutRc::own(Some(<_>::default())),
    }
  }
}
#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use crate::{
    prelude::*,
    test::{MockBox, MockMulti},
    widget_tree::WidgetTree,
  };

  #[test]
  fn full_lifecycle() {
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
        DynWidget {
          dyns: trigger.then(|| widget! {
            MockBox {
              size: Size::zero(),
              on_mounted: move |_| lifecycle.silent().push("dyn mounted"),
              on_performed_layout: move |_| lifecycle.silent().push("dyn performed layout"),
              on_disposed: move |_| lifecycle.silent().push("dyn disposed")
            }
          })
        }
      }
    };

    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(w, WindowCtx::new(AppContext::default(), scheduler));
    assert_eq!(&**lifecycle.state_ref(), ["static mounted"]);
    tree.layout(Size::new(100., 100.));
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
    tree.layout(Size::zero());
    assert_eq!(
      &**lifecycle.state_ref(),
      [
        "static mounted",
        "dyn mounted",
        "dyn performed layout",
        "static performed layout",
        "dyn disposed",
      ]
    );
  }

  #[test]
  fn track_lifecycle() {
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
        DynWidget {
          dyns: (0..*cnt).map(move |_| widget! {
            MockBox {
              size: Size::zero(),
              on_mounted: move |ctx| {
                mounted.insert(ctx.id);
              },
              on_disposed: move |ctx| {
                disposed.insert(ctx.id);
              },
            }
          })
        }
      }
    };

    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(w, WindowCtx::new(AppContext::default(), scheduler));
    tree.layout(Size::new(100., 100.));
    let mounted_ids = (*mounted.state_ref()).clone();

    *cnt.state_ref() = 5;
    tree.layout(Size::zero());

    let disposed_ids = (*disposed.state_ref()).clone();
    assert_eq!(mounted_ids.len(), 3);
    assert_eq!(mounted_ids, disposed_ids);
  }
}
