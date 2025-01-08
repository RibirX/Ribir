//! Providers serve as a mechanism in Ribir to distribute data to descendants.
//!
//! The data is set up in the context for the descendants' scope and removed
//! when out of scope. Descendants can access the data using [`Provider::of`]
//! with contexts such as `BuildCtx`, `LayoutCtx`, `PaintingCtx`, and event
//! objects.
//!
//! ```
//! use ribir::prelude::*;
//!
//! providers! {
//!   providers: [Provider::new(1i32)],
//!   @{
//!     // Providers accessible via `BuildCtx`.
//!     let value = Provider::of::<i32>(BuildCtx::get()).unwrap();
//!     assert_eq!(*value, 1);
//!     @Text {
//!       text: "Good!",
//!       on_tap: move |e| {
//!         // Accessing providers through event objects.
//!         let value = Provider::write_of::<i32>(e).unwrap();
//!         assert_eq!(*value, 1);
//!       }
//!     }
//!   }
//! };
//! ```
//!
//! ## Provider Based on Type
//!
//! Providers are based on their type, allowing only one provider of the same
//! type to remain in the context at a time. When a new provider of the same
//! type is provided, it overwrites the previous one. Thus, the closest provider
//! of the same type to its usage will be accessed.
//!
//! ```
//! use ribir::prelude::*;
//! use smallvec::smallvec;
//!
//! providers! {
//!   providers: smallvec![Provider::new(1i32), Provider::new(Color::RED)],
//!   @Providers {
//!     providers: [Provider::new(2i32)],
//!     @ {
//!       let value = Provider::of::<i32>(BuildCtx::get()).unwrap();
//!       assert_eq!(*value, 2);
//!       let value = Provider::of::<Color>(BuildCtx::get()).unwrap();
//!       assert_eq!(*value, Color::RED);
//!       @Text { text: "`i32` overwritten with 2, color remains unchanged" }
//!     }
//!   }
//! };
//! ```
//!
//! State can be provided as either its value or itself. When providing a writer
//! as its value, you can access its write reference using
//! [`Provider::write_of`] to modify the state.
//!
//! ```
//! use ribir::prelude::*;
//! use smallvec::smallvec;
//!
//! let state = Stateful::new(1i32);
//!
//! providers! {
//!   providers: smallvec![
//!     // Providing a `Stateful<i32>`
//!     Provider::new(state.clone_writer()),
//!     // Providing an `i32`
//!     Provider::value_of_state(state)
//!   ],
//!   @ {
//!     let ctx = BuildCtx::get();
//!     // Accessing the state value
//!     let value = *Provider::of::<i32>(ctx).unwrap();
//!     assert_eq!(value, 1);
//!     // Accessing the state itself
//!     {
//!       let _state = Provider::of::<Stateful<i32>>(ctx).unwrap();
//!     }
//!     // Accessing the write reference of the state to modify the value
//!     let mut value = Provider::write_of::<i32>(ctx).unwrap();
//!     *value = 2;
//!     @Text { text: "Both state and its value provided!" }
//!   }
//! };
//! ```
//!
//! ## Scope of Providers in the Build Process
//!
//! Providers are visible to their descendants. However, in the build process,
//! the scope starts when the `Providers` are created and ends when they are
//! fully composed with their children. Thus, it is recommended to declare
//! providers together with their children.
//!
//! ```
//! use ribir::prelude::*;
//!
//! let _bad_example = fn_widget! {
//!   let providers = Providers::new([Provider::new(1i32)]);
//!   // Providers are already accessible here
//!   let v = *Provider::of::<i32>(BuildCtx::get()).unwrap();
//!   assert_eq!(v, 1);
//!
//!   @Row {
//!     @Text { text: "I can access providers in the build process" }
//!     @ $providers {
//!       @Text { text: "We only want providers accessible in this subtree" }
//!     }
//!   }
//! };
//! ```
//!
//! The correct approach is as follows:
//!
//! ```
//! use ribir::prelude::*;
//!
//! let _good_example = fn_widget! {
//!   @Row {
//!     @Text { text: "I can't see the `i32` provider" }
//!     @Providers {
//!       providers: [Provider::new(1i32)],
//!       @Text { text: "I can see the `i32` provider" }
//!     }
//!   }
//! };
//! ```
use std::cell::RefCell;

use ribir_painter::Color;
use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::prelude::*;

/// The widget that provides data to its descendants. See the
/// [module-level](self) documentation for more details.
pub struct Providers {
  providers: RefCell<SmallVec<[Provider; 1]>>,
}

/// Macro used to generate a function widget using `BuildVariants` as the root
/// widget.
#[macro_export]
macro_rules! providers {
  ($($t: tt)*) => { fn_widget! { @Providers { $($t)* } } };
}

/// The type use to store the data you want to share.
pub enum Provider {
  /// The value of the provider has not been setup yet.
  Setup(Box<dyn ProviderSetup>),
  /// The provider has already been setup to the context, and wait for restore.
  Restore(Box<dyn ProviderRestore>),
}

/// This trait is used to set up the providers in the context. In most cases,
/// you don't need to worry about it unless you want to customize the setup
/// process.
pub trait ProviderSetup {
  fn setup(self: Box<Self>, ctx: &mut ProviderCtx) -> Box<dyn ProviderRestore>;
}

/// This trait is used to retrieve the providers from the context. In most
/// cases, you don't need to worry about it unless you want to customize the
/// retrieval process.
pub trait ProviderRestore {
  fn restore(self: Box<Self>, ctx: &mut ProviderCtx) -> Box<dyn ProviderSetup>;
}

/// A type for providing the container color of the widget.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ContainerColor(pub Color);

/// The context used to store the providers.
#[derive(Default)]
pub struct ProviderCtx {
  data: ahash::AHashMap<TypeInfo, Box<dyn Query>>,
  /// The stack is used to temporarily store the providers that are set up and
  /// will be popped and restored when the scope is exited.
  setup_providers: Vec<(WidgetId, *const Providers)>,
}

impl Provider {
  /// Establish a provider for `T`.
  pub fn new<T: 'static>(value: T) -> Provider { Provider::Setup(Box::new(Setup::new(value))) }

  /// Establish a provider for the `Value` of a state. If you use a writer to
  /// create this provider, a write reference of the value can be accessed
  /// through [`Provider::write_of`].
  pub fn value_of_state<V: 'static>(value: impl StateReader<Value = V> + Query) -> Provider {
    Provider::Setup(Box::new(Setup::from_state(value)))
  }

  /// Access the provider of `P` within the context.
  pub fn of<P: 'static>(ctx: &impl AsRef<ProviderCtx>) -> Option<QueryRef<P>> {
    ctx.as_ref().get_provider::<P>()
  }

  /// Access the write reference of `P` within the context.
  pub fn write_of<P: 'static>(ctx: &impl AsRef<ProviderCtx>) -> Option<WriteRef<P>> {
    ctx.as_ref().get_provider_write::<P>()
  }

  pub(crate) fn custom(info: TypeInfo, value: Box<dyn Query>) -> Self {
    Provider::Setup(Box::new(Setup { info, value }))
  }

  /// Setup the provider to the context.
  pub fn setup(&mut self, ctx: &mut ProviderCtx) {
    let Provider::Setup(setup) = self else {
      panic!("Provider already setup");
    };
    // Safety: We will have two references to the setup, but we will
    // forget one of them after the setup is completed.
    let setup = unsafe { Box::from_raw(&mut **setup) };
    let restore = setup.setup(ctx);
    let f = std::mem::replace(self, Provider::Restore(restore));
    std::mem::forget(f);
  }

  /// Restore the provider from the context.
  pub fn restore(&mut self, map: &mut ProviderCtx) {
    let Provider::Restore(restore) = self else {
      panic!("Provider restore not match.");
    };
    // Safety: We will have two references to the restore, but we will forget
    // one of them after the restore is completed.
    let restore = unsafe { Box::from_raw(&mut **restore) };
    let setup = restore.restore(map);
    let f = std::mem::replace(self, Provider::Setup(setup));
    std::mem::forget(f);
  }

  fn info<T: 'static>() -> TypeInfo { TypeInfoOf::<T>::type_info() }
}

pub struct ProvidersDeclarer {
  providers: Option<SmallVec<[Provider; 1]>>,
}

impl Declare for Providers {
  type Builder = ProvidersDeclarer;

  fn declarer() -> Self::Builder { ProvidersDeclarer { providers: None } }
}

impl ProvidersDeclarer {
  pub fn providers(mut self, variants: impl Into<SmallVec<[Provider; 1]>>) -> Self {
    assert!(self.providers.is_none(), "Providers already initialized");
    self.providers = Some(variants.into());
    self
  }
}

impl ObjDeclarer for ProvidersDeclarer {
  type Target = Providers;

  #[track_caller]
  fn finish(self) -> Self::Target {
    let Some(mut providers) = self.providers else {
      panic!("Providers not initialized");
    };
    let map = BuildCtx::get_mut().as_mut();

    for p in providers.iter_mut() {
      p.setup(map);
    }
    Providers { providers: RefCell::new(providers) }
  }
}

impl Providers {
  pub fn new(providers: impl Into<SmallVec<[Provider; 1]>>) -> Self {
    Providers::declarer()
      .providers(providers)
      .finish()
  }

  pub(crate) fn setup_providers(&self, map: &mut ProviderCtx) {
    for p in self.providers.borrow_mut().iter_mut() {
      p.setup(map);
    }
  }

  pub(crate) fn restore_providers(&self, map: &mut ProviderCtx) {
    for p in self.providers.borrow_mut().iter_mut() {
      p.restore(map);
    }
  }
}

impl ContainerColor {
  pub fn provider(color: Color) -> Provider { Provider::new(ContainerColor(color)) }
}

impl ProviderCtx {
  pub(crate) fn collect_from(id: WidgetId, tree: &WidgetTree) -> ProviderCtx {
    let ancestors = id
      .ancestors(tree)
      .filter(|id| id.queryable(tree))
      .collect::<Vec<_>>();

    let mut ctx = ProviderCtx::default();
    let mut providers = SmallVec::new();
    for p in ancestors.iter().rev() {
      ctx.push_providers_for(*p, tree, &mut providers);
    }

    ctx
  }

  /// Push the providers to the stack, the caller should guarantee that the
  /// providers is available before popping it.
  pub(crate) fn push_providers(&mut self, id: WidgetId, providers: *const Providers) {
    unsafe { &*providers }.setup_providers(self);
    self.setup_providers.push((id, providers));
  }

  /// Pop the providers from the stack and restore it.
  pub(crate) fn pop_providers(&mut self) -> Option<(WidgetId, *const Providers)> {
    self.setup_providers.pop().inspect(|(_, p)| {
      unsafe { &**p }.restore_providers(self);
    })
  }

  /// Pop the providers for the specified widget from the stack and restore it.
  ///
  /// Only if the `w` is the last widget in the stack, it will be invoked.
  pub(crate) fn pop_providers_for(&mut self, w: WidgetId) {
    while self
      .setup_providers
      .last()
      .is_some_and(|(id, _)| id == &w)
    {
      self.pop_providers();
    }
  }

  pub(crate) fn push_providers_for<'t>(
    &mut self, w: WidgetId, tree: &'t WidgetTree, buffer: &mut SmallVec<[QueryHandle<'t>; 1]>,
  ) {
    w.assert_get(tree)
      .query_all(&QueryId::of::<Providers>(), buffer);

    for providers in buffer
      .drain(..)
      .rev()
      .filter_map(QueryHandle::into_ref::<Providers>)
    {
      self.push_providers(w, &*providers);
    }
  }

  pub(crate) fn remove_raw_provider(&mut self, info: &TypeInfo) -> Option<Box<dyn Query>> {
    self.data.remove(info)
  }

  pub(crate) fn set_raw_provider(
    &mut self, info: TypeInfo, p: Box<dyn Query>,
  ) -> Option<Box<dyn Query>> {
    self.data.insert(info, p)
  }

  pub(crate) fn get_raw_provider(&self, info: &TypeInfo) -> Option<&dyn Query> {
    self.data.get(info).map(|q| &**q)
  }

  pub(crate) fn get_provider<T: 'static>(&self) -> Option<QueryRef<T>> {
    let info = Provider::info::<T>();
    self
      .data
      .get(&info)
      .and_then(|q| q.query(&QueryId::of::<T>()))
      .and_then(QueryHandle::into_ref)
  }

  pub(crate) fn get_provider_write<T: 'static>(&self) -> Option<WriteRef<T>> {
    let info = Provider::info::<T>();
    self
      .data
      .get(&info)
      .and_then(|q| q.query_write(&QueryId::of::<T>()))
      .and_then(QueryHandle::into_mut)
  }

  pub(crate) fn remove_key_value_if(
    &mut self, f: impl Fn(&TypeInfo) -> bool,
  ) -> Vec<(TypeInfo, Box<dyn Query>)> {
    let mut out = Vec::new();
    let keys = self.data.keys().cloned().collect::<Vec<_>>();
    for k in keys {
      if f(&k) {
        if let Some(v) = self.data.remove(&k) {
          out.push((k, v));
        }
      }
    }
    out
  }
}

impl Providers {
  pub fn with_child<'w, const M: usize>(self, child: impl IntoWidget<'w, M>) -> Widget<'w> {
    let child = child.into_widget();
    self.restore_providers(BuildCtx::get_mut().as_mut());
    Widget::from_fn(move |ctx| {
      self.setup_providers(ctx.as_mut());
      let id = ctx.build(child);
      self.restore_providers(ctx.as_mut());
      id.wrap_node(ctx.tree_mut(), |render| {
        Box::new(ProvidersRender { providers: Queryable(self), render })
      });
      id
    })
  }
}

impl Drop for Providers {
  fn drop(&mut self) {
    let need_restore = self
      .providers
      .borrow()
      .iter()
      .any(|p| matches!(p, Provider::Restore(_)));

    assert!(
      !need_restore,
      "You have created a `Providers` object but did not use it to wrap a child. This may result \
       in the providers context being in an incorrect state."
    );
  }
}

impl Drop for ProviderCtx {
  fn drop(&mut self) {
    while self.pop_providers().is_some() {}

    assert!(
      self.data.is_empty(),
      "Some providers may not be restored if you create an independent `Providers` instead of \
       composing it with a child."
    );
  }
}

struct ProvidersRender {
  providers: Queryable<Providers>,
  render: Box<dyn RenderQueryable>,
}

impl Query for ProvidersRender {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all(query_id, out);
    if let Some(h) = self.providers.query(query_id) {
      out.push(h)
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all_write(query_id, out);
    if let Some(h) = self.providers.query_write(query_id) {
      out.push(h)
    }
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self
      .providers
      .query(query_id)
      .or_else(|| self.render.query(query_id))
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self
      .providers
      .query_write(query_id)
      .or_else(|| self.render.query_write(query_id))
  }

  fn queryable(&self) -> bool { true }
}

impl Render for ProvidersRender {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let Self { render, providers: Queryable(p) } = self;
    p.setup_providers(ctx.as_mut());
    let size = render.perform_layout(clamp, ctx);
    p.restore_providers(ctx.as_mut());
    size
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let Self { render, providers: Queryable(p) } = self;
    let id = ctx.id();
    // The providers will be popped in the `PaintingCtx::finish` method, once the
    // painting of the entire subtree is completed.
    ctx.as_mut().push_providers(id, p);

    render.paint(ctx);
  }

  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    let Self { render, providers: Queryable(p) } = self;
    let id = ctx.id();
    // The providers will be popped in the `HitTestCtx::finish` method, once the
    // hit test of the entire subtree is completed.
    ctx.as_mut().push_providers(id, p);
    render.hit_test(ctx, pos)
  }

  fn only_sized_by_parent(&self) -> bool { self.render.only_sized_by_parent() }

  fn get_transform(&self) -> Option<Transform> { self.render.get_transform() }
}

pub struct Setup {
  info: TypeInfo,
  value: Box<dyn Query>,
}

struct Restore {
  info: TypeInfo,
  value: Option<Box<dyn Query>>,
}

impl ProviderSetup for Setup {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let Setup { info, value } = *self;
    let old = map.set_raw_provider(info, value);

    Box::new(Restore { info, value: old })
  }
}

impl ProviderRestore for Restore {
  fn restore(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderSetup> {
    let Restore { info, value } = *self;
    let v = if let Some(v) = value {
      map.set_raw_provider(info, v)
    } else {
      map.remove_raw_provider(&info)
    };
    let Some(v) = v else {
      panic!("Provider restore not matched");
    };
    Box::new(Setup { info, value: v })
  }
}

impl Setup {
  pub(crate) fn new<T: 'static>(value: T) -> Self {
    Setup { info: Provider::info::<T>(), value: Box::new(Queryable(value)) }
  }

  pub(crate) fn from_state<V: 'static>(value: impl StateReader<Value = V> + Query) -> Self {
    Setup { info: Provider::info::<V>(), value: Box::new(value) }
  }
}

#[cfg(test)]
mod tests {

  use smallvec::smallvec;

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[test]
  fn smoke() {
    reset_test_env!();

    let mut wnd = TestWindow::new(mock_multi! {
      @Providers {
        providers: smallvec![Provider::new(Color::RED)],
        @ {
          assert_eq!(BuildCtx::color(), Color::RED);
          @MockMulti {
            @fn_widget!{
              assert_eq!(BuildCtx::color(), Color::RED);
              Void
            }
          }
        }
      }
      @ {
        let color = BuildCtx::color();
        assert_eq!(color, Palette::of(BuildCtx::get()).primary());
        Void
      }
    });
    wnd.draw_frame();
  }

  #[test]
  fn embedded() {
    reset_test_env!();

    let mut wnd = TestWindow::new(providers! {
      providers: smallvec![Provider::new(Color::RED)],
      @Providers {
        providers: smallvec![ContainerColor::provider(Color::GREEN)],
        @ {
          let container_color = BuildCtx::container_color();
          assert_eq!(container_color, Color::GREEN);
          let color = BuildCtx::color();
          assert_eq!(color, Color::RED);
          Void
        }
      }
    });
    wnd.draw_frame();
  }

  #[test]
  fn direct_pass() {
    reset_test_env!();

    let (value, w_value) = split_value(0);

    let mut wnd = TestWindow::new(providers! {
      providers: smallvec![Provider::new(1i32)],
      @{
        let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
        *w_value.write() = *v;
        Void
      }
    });
    wnd.draw_frame();
    assert_eq!(*value.read(), 1);
  }

  #[test]
  fn indirect_pass() {
    reset_test_env!();

    let (value, w_value) = split_value(0);
    let w = providers! {
      providers: smallvec![Provider::new(1i32)],
      @MockBox {
        size: Size::new(1.,1.),
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          *$w_value.write() = *v;
          Void
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*value.read(), 1);
  }

  #[test]
  fn with_multi_providers() {
    reset_test_env!();

    let (value1, w_value1) = split_value(0);
    let (value2, w_value2) = split_value(0);
    let w = mock_multi! {
      @Providers {
        providers: smallvec![Provider::new(1i32)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          *$w_value1.write() = *v;
          Void
        }
      }

      @Providers {
        providers: smallvec![Provider::new(2i32)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          *$w_value2.write() = *v;
          Void
        }
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    assert_eq!(*value1.read(), 1);
    assert_eq!(*value2.read(), 2);
  }

  #[test]
  fn provider_for_pipe() {
    reset_test_env!();
    let (value, w_value) = split_value(0);
    let (trigger, w_trigger) = split_value(true);

    let w = providers! {
      providers: smallvec![Provider::new(w_value.clone_writer())],
      @ {
        // We do not allow the use of the build context in the pipe at the moment.
        let value = Provider::of::<Stateful<i32>>(BuildCtx::get())
          .unwrap().clone_writer();
        pipe!(*$trigger).map(move |_| {
          *value.write() += 1;
          Void
        })
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    assert_eq!(*value.read(), 1);

    *w_trigger.write() = false;
    wnd.draw_frame();
    assert_eq!(*value.read(), 2);
  }
}
