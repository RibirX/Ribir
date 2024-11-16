use smallvec::SmallVec;

use crate::prelude::*;

/// This widget enables its descendants to access the data it provides,
/// streamlining data sharing throughout the widget tree.
///
/// Descendants have the ability to inquire about the type of data provided by
/// their ancestors. If the ancestor is a writer, descendants can also access
/// the write reference (`WriteRef`) for that data.
///
/// Its child must be a function widget, which enforces its subtree to utilize
/// the build context it provides for construction.
///
/// Data querying occurs from the bottom to the top of the widget tree. In cases
/// where there are two providers of the same type in one path, the closer
/// provider will be queried.
///
/// The system theme should serve as a global provider by default.
///
/// You can utilize the provider with `BuildCtx`, the event object, `LayoutCtx`,
/// and `PaintCtx`.
///
/// ## Example
///
/// Any type can be wrapped with `Queryable` for providing data.
///
/// ```rust
/// use ribir::prelude::*;
///
/// Provider::new(Box::new(Queryable(1i32)))
///   // Provider only accepts function widgets as its child.
///   .with_child(fn_widget! {
///     let value = Provider::of::<i32>(BuildCtx::get()).unwrap();
///     assert_eq!(*value, 1);
///
///     let value = Provider::write_of::<i32>(BuildCtx::get());
///     // We not share a writer.
///     assert!(value.is_none());
///     @Text { text: "Good!" }
///   });
/// ```
///
/// You can provide a state reader or writer without the `Queryable` wrapper. If
/// you provide a writer, you can access its write reference to modify it.
///
/// ```rust
/// use ribir::prelude::*;
///
/// Provider::new(Box::new(Stateful::new(0i32))).with_child(fn_widget! {
///   // we can query the type of the data.
///   let ctx = BuildCtx::get();
///   {
///     let cnt = Provider::of::<Stateful<i32>>(ctx).unwrap();
///     assert_eq!(*cnt.read(), 0);
///   }
///
///   // the write ref of the value
///   {
///     let mut cnt: WriteRef<i32> = Provider::write_of::<i32>(ctx).unwrap();
///     assert_eq!(*cnt, 0);
///     *cnt = 1;
///   }
///
///   // The value type of the state.
///   let cnt = Provider::of::<i32>(ctx).unwrap();
///   assert_eq!(*cnt, 1);
///   @Text { text: "Good!" }
/// });
/// ```
#[simple_declare]
pub struct Provider {
  pub provider: Box<dyn Query>,
}

/// Macro use to create a `Provider` that provides many data.
///
/// ```
/// use ribir::prelude::*;
///
/// providers![State::value(0), Queryable("Hello!")].with_child(fn_widget! {
///   let hi = Provider::of::<&'static str>(BuildCtx::get()).unwrap();
///   @Text { text: *hi }
/// });
/// ```
#[macro_export]
macro_rules! providers {
  ($($q: expr),*) => {
    Provider::new(Box::new([$(Box::new($q) as Box<dyn Query>),*]))
  };
}

impl Provider {
  /// Create a Provider
  #[inline]
  pub fn new(provider: Box<dyn Query>) -> Self { Provider { provider } }

  /// Query a reference of type `T` if it was provided by the ancestors.
  #[inline]
  pub fn of<T: 'static>(ctx: &impl ProviderCtx) -> Option<QueryRef<T>> { ctx.provider_of() }

  /// Query a write reference of type `T` if the ancestor provided a writer of
  /// type `T`.
  #[inline]
  pub fn write_of<T: 'static>(ctx: &impl ProviderCtx) -> Option<WriteRef<T>> {
    ctx.provider_write_of()
  }
}

impl<'c> ComposeChild<'c> for Provider {
  type Child = FnWidget<'c>;
  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let provider = this
      .try_into_value()
      .unwrap_or_else(|_| {
        panic!(
          "Provider should not be treated as a shared object to be held onto; instead, utilize \
           `Provider::xxx_of` to access its content."
        )
      })
      .provider;

    // We push the provider into the build context to ensure that the widget build
    // logic can access this provider.
    let ctx = BuildCtx::get_mut();
    ctx.current_providers.push(provider);
    child.into_widget().on_build(|id| {
      let provider = ctx.current_providers.pop().unwrap();
      id.attach_data(provider, ctx.tree_mut());
    })
  }
}

/// The context allows `Provider` to access shared data. It is implemented for
/// `BuildCtx` and other widget contexts such as `LayoutCtx`, `PaintCtx`, and
/// event objects.
pub trait ProviderCtx {
  fn all_providers<Q: 'static>(&self) -> impl Iterator<Item = QueryRef<Q>>;

  fn all_write_providers<Q: 'static>(&self) -> impl Iterator<Item = WriteRef<Q>>;

  fn provider_of<Q: 'static>(&self) -> Option<QueryRef<Q>> { self.all_providers().next() }

  fn provider_write_of<Q: 'static>(&self) -> Option<WriteRef<Q>> {
    self.all_write_providers().next()
  }
}

impl BuildCtx {
  fn current_providers<Q: 'static>(&self) -> impl Iterator<Item = QueryRef<Q>> {
    self
      .current_providers
      .iter()
      .rev()
      .filter_map(|p| p.query(&QueryId::of::<Q>()))
      .filter_map(QueryHandle::into_ref)
  }

  fn current_write_providers<Q: 'static>(&self) -> impl Iterator<Item = WriteRef<Q>> {
    self
      .current_providers
      .iter()
      .rev()
      .filter_map(|p| p.query_write(&QueryId::of::<Q>()))
      .filter_map(QueryHandle::into_mut)
  }
}

impl ProviderCtx for BuildCtx {
  fn all_providers<Q: 'static>(&self) -> impl Iterator<Item = QueryRef<Q>> {
    self.current_providers::<Q>().chain(
      self
        .providers
        .iter()
        .rev()
        .filter_map(|id| id.query_ref(self.tree())),
    )
  }

  fn all_write_providers<Q: 'static>(&self) -> impl Iterator<Item = WriteRef<Q>> {
    self.current_write_providers::<Q>().chain(
      self
        .providers
        .iter()
        .rev()
        .filter_map(|id| id.query_write(self.tree())),
    )
  }
}

impl<T: Deref<Target: WidgetCtxImpl>> ProviderCtx for T {
  fn all_providers<Q: 'static>(&self) -> impl Iterator<Item = QueryRef<Q>> {
    let tree = self.tree();
    self
      .id()
      .ancestors(tree)
      .filter_map(|id| id.query_ref(tree))
  }

  fn all_write_providers<Q: 'static>(&self) -> impl Iterator<Item = WriteRef<Q>> {
    let tree = self.tree();
    self
      .id()
      .ancestors(tree)
      .filter_map(|id| id.query_write(tree))
  }
}

impl<const M: usize> Query for [Box<dyn Query>; M] {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self
      .iter()
      .for_each(|q| q.query_all(query_id, out))
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self.iter().find_map(|q| q.query(query_id))
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self.iter().find_map(|q| q.query_write(query_id))
  }

  fn queryable(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn direct_pass() {
    reset_test_env!();
    let (value, w_value) = split_value(0);

    let w = fn_widget! {
      let w_value = w_value.clone_writer();
      Provider::new(Box::new(Queryable(1i32))).with_child(fn_widget! {
        let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
        *w_value.write() = *v;
        Void
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*value.read(), 1);
  }

  #[test]
  fn indirect_pass() {
    reset_test_env!();

    let (value, w_value) = split_value(0);
    let w = fn_widget! {
      let w_value = w_value.clone_writer();
      Provider::new(Box::new(Queryable(1i32))).with_child(fn_widget! {
        @MockBox {
          size: Size::new(1.,1.),
          @ {
            let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
            *$w_value.write() = *v;
            Void
          }
        }
      })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*value.read(), 1);
  }

  #[test]
  fn provider_for_pipe() {
    reset_test_env!();
    let (value, w_value) = split_value(0);
    let (trigger, w_trigger) = split_value(true);

    let w = fn_widget! {
      let trigger = trigger.clone_watcher();
      Provider::new(Box::new(w_value.clone_writer()))
        .with_child(fn_widget! {
          // We do not allow the use of the build context in the pipe at the moment.
          let value = Provider::of::<Stateful<i32>>(BuildCtx::get())
            .unwrap().clone_writer();
          pipe!(*$trigger).map(move |_| {
            *value.write() += 1;
            Void
          })
        })
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*value.read(), 1);

    *w_trigger.write() = false;
    wnd.draw_frame();
    assert_eq!(*value.read(), 2);
  }
}
