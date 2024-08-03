use ribir_algo::Sc;
use widget_id::{new_node, RenderQueryable};

use crate::{prelude::*, render_helper::PureRender};

/// This widget enables its descendants to access the data it provides,
/// streamlining data sharing throughout the widget tree. Descendants have the
/// ability to inquire about the type of data provided by their ancestors. If
/// the ancestor is a writer, descendants can also access the write reference
/// (`WriteRef`) for that data.

/// Its child must be a function widget, which enforces its subtree to utilize
/// the build context it provides for construction.
///
/// Data querying occurs from the bottom to the top of the widget tree. In cases
/// where there are two providers of the same type in one path, the closer
/// provider will be queried.

/// The system theme should serve as a global provider by default.
///
/// You can utilize the provider with `BuildCtx`, the event object, `LayoutCtx`,
/// and `PaintCtx`.

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
///     let value = Provider::of::<i32>(ctx!()).unwrap();
///     assert_eq!(*value, 1);
///
///     let value = Provider::write_of::<i32>(ctx!());
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
///   {
///     let cnt = Provider::of::<Stateful<i32>>(ctx!()).unwrap();
///     assert_eq!(*cnt.read(), 0);
///   }
///
///   // the write ref of the value
///   {
///     let mut cnt: WriteRef<i32> = Provider::write_of::<i32>(ctx!()).unwrap();
///     assert_eq!(*cnt, 0);
///     *cnt = 1;
///   }
///
///   // The value type of the state.
///   let cnt = Provider::of::<i32>(ctx!()).unwrap();
///   assert_eq!(*cnt, 1);
///   @Text { text: "Good!" }
/// });
/// ```
#[simple_declare]
pub struct Provider {
  #[declare(custom)]
  pub provider: Box<dyn Query>,
}

/// Macro use to create a `Provider` that provides many data.
///
/// ```
/// use ribir::prelude::*;
///
/// providers![State::value(0), Queryable("Hello!")].with_child(fn_widget! {
///   let hi = Provider::of::<&'static str>(ctx!()).unwrap();
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

impl ProviderDeclarer {
  pub fn provider(mut self, p: impl Query) -> Self {
    self.provider = Some(Box::new(p));
    self
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
           Provider::xxx_of to access its content.",
        );
      })
      .provider;

    let f = move |ctx: &mut BuildCtx| {
      if ctx.pre_alloc_id.is_none() {
        let node: Box<dyn RenderQueryable> = Box::new(PureRender(Void));
        let id = new_node(&mut ctx.tree_mut().arena, node);
        ctx.pre_alloc_id = Some(id);
      }
      let alloc_id = ctx.pre_alloc_id.clone().unwrap();
      ctx.startup = alloc_id;

      // This provider needs to be visible to the child, so we must attach it to the
      // pre-allocated node first, allowing its build logic to access this provider.
      let provider = PreAttachedProvider::new(provider);
      alloc_id.attach_data(Box::new(provider.clone()), ctx.tree_mut());
      ctx.providers.push(alloc_id);

      // We need to consume the root widget first and attach it after the entire
      // widget build is finished. Otherwise, the provider may be attached more
      // internally than it should be.
      let (id, child) = child(ctx).build_root(ctx);

      ctx.providers.pop();
      assert_eq!(alloc_id, id);
      id.attach_data(provider.into_inner(), ctx.tree_mut());

      child
    };
    f.into_widget()
  }
}

/// The context allows `Provider` to access shared data. It is implemented for
/// `BuildCtx` and other widget contexts such as `LayoutCtx`, `PaintCtx`, and
/// event objects.
pub trait ProviderCtx {
  fn provider_of<Q: 'static>(&self) -> Option<QueryRef<Q>>;
  fn provider_write_of<Q: 'static>(&self) -> Option<WriteRef<Q>>;
}

// todo:
// - System theme attach to every root of window.

impl ProviderCtx for BuildCtx {
  fn provider_of<T: 'static>(&self) -> Option<QueryRef<T>> {
    self
      .providers
      .iter()
      .find_map(|id| id.query_ref(self.tree()))
  }

  fn provider_write_of<T: 'static>(&self) -> Option<WriteRef<T>> {
    self
      .providers
      .iter()
      .find_map(|id| id.query_write(self.tree()))
  }
}

impl<T: Deref<Target: WidgetCtxImpl>> ProviderCtx for T {
  fn provider_of<Q: 'static>(&self) -> Option<QueryRef<Q>> {
    widget_ctx_queryable_ancestors(self).find_map(|id| id.query_ref(self.tree()))
  }
  fn provider_write_of<Q: 'static>(&self) -> Option<WriteRef<Q>> {
    widget_ctx_queryable_ancestors(self).find_map(|id| id.query_write(self.tree()))
  }
}

fn widget_ctx_queryable_ancestors(
  ctx: &impl Deref<Target: WidgetCtxImpl>,
) -> impl Iterator<Item = WidgetId> + '_ {
  let tree = ctx.tree();
  ctx
    .id()
    .ancestors(tree)
    .filter(|id| id.queryable(tree))
}

impl<const M: usize> Query for [Box<dyn Query>; M] {
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    self
      .iter()
      .flat_map(|q| q.query_all(type_id).into_iter())
      .collect()
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    self.iter().find_map(|q| q.query(type_id))
  }

  fn queryable(&self) -> bool { true }
}

#[derive(Clone)]
struct PreAttachedProvider(Sc<Box<dyn Query>>);

impl PreAttachedProvider {
  fn new(p: Box<dyn Query>) -> Self { Self(Sc::new(p)) }

  fn into_inner(self) -> Box<dyn Query> {
    Sc::try_unwrap(self.0)
      .unwrap_or_else(|_| panic!("The pre-attached provider needs to be released."))
  }
}

impl Query for PreAttachedProvider {
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    self.0.query_all(type_id)
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.0.query(type_id) }

  fn queryable(&self) -> bool { true }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::{reset_test_env, test_helper::*};

  #[test]
  fn direct_pass() {
    reset_test_env!();
    let value = Stateful::new(0);
    let c_v = value.clone_writer();
    let w = Provider::new(Box::new(Queryable(1i32))).with_child(fn_widget! {
      let v = Provider::of::<i32>(ctx!()).unwrap();
      *c_v.write() = *v;
      Void
    });

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*value.read(), 1);
  }

  #[test]
  fn indirect_pass() {
    reset_test_env!();
    let value = Stateful::new(0);
    let c_v = value.clone_writer();
    let w = Provider::new(Box::new(Queryable(1i32))).with_child(fn_widget! {
      @MockBox {
        size: Size::new(1.,1.),
        @ {
          let v = Provider::of::<i32>(ctx!()).unwrap();
          *c_v.write() = *v;
          Void
        }
      }
    });

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*value.read(), 1);
  }

  #[test]
  fn provider_for_pipe() {
    reset_test_env!();
    let (watcher, writer) = split_value(0);
    let (t_watcher, t_writer) = split_value(true);

    let w = Provider::new(Box::new(writer.clone_writer())).with_child(fn_widget! {
      pipe!(*$t_watcher).map(move |_| {
        let mut v = Provider::write_of::<i32>(ctx!()).unwrap();
        *v += 1;
        Void
      })
    });

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*watcher.read(), 1);

    *t_writer.write() = false;
    wnd.draw_frame();
    assert_eq!(*watcher.read(), 2);
  }
}
