//! Providers enable hierarchical data distribution in Ribir, allowing data to
//! be made available to descendant widgets within a specific scope.
//!
//! Providers establish data in the context for descendant widgets. The data is
//! automatically scoped - it becomes available when entering the provider's
//! scope and is removed when exiting that scope. Descendants can access
//! provided data through various contexts (`BuildCtx`, `LayoutCtx`,
//! `PaintingCtx`) and event objects.
//!
//! ## Basic Usage
//!
//! ```
//! use ribir_core::prelude::*;
//!
//! providers! {
//!   providers: [Provider::new(1i32)],
//!   @{
//!     // Access provider via BuildCtx
//!     let value = Provider::of::<i32>(BuildCtx::get()).unwrap();
//!     assert_eq!(*value, 1);
//!
//!     @Text {
//!       text: "Good!",
//!       on_tap: move |e| {
//!         // Access provider through event object
//!         let mut value = Provider::write_of::<i32>(e).unwrap();
//!         *value = 2; // Modify the value
//!       }
//!     }
//!   }
//! };
//! ```
//!
//! ## Type-Based Provider Resolution
//!
//! Providers are uniquely identified by their type. Only one provider of a
//! given type can exist in any context. When a new provider of the same type is
//! introduced, it shadows previous providers. Descendants always access the
//! closest provider in their ancestor hierarchy.
//!
//! ```
//! use ribir_core::prelude::*;
//! use smallvec::smallvec;
//!
//! providers! {
//!   providers: smallvec![Provider::new(1i32), Provider::new(Color::RED)],
//!   @Providers {
//!     providers: [Provider::new(2i32)], // Shadows outer i32 provider
//!     @ {
//!       let ctx = BuildCtx::get();
//!       assert_eq!(*Provider::of::<i32>(ctx).unwrap(), 2);
//!       assert_eq!(*Provider::of::<Color>(ctx).unwrap(), Color::RED);
//!       @Text { text: "Inner i32 shadows outer, Color remains visible" }
//!     }
//!   }
//! };
//! ```
//!
//! ## State Providers
//!
//! State can be accessed through different provider types:
//! - **Reader**: Immutable access (`Provider::reader`)
//! - **Watcher**: Immutable access with change notifications
//!   (`Provider::watcher`)
//! - **Writer**: Mutable access (`Provider::writer`)
//!
//! ### Access Methods
//!
//! | Provider Type | Read Reference   | Write Reference      | State Access          |
//! |---------------|------------------|----------------------|-----------------------|
//! | Reader        | `Provider::of`   | —                    | `Provider::reader_of` |
//! | Watcher       | `Provider::of`   | —                    | `Provider::watcher_of`|
//! | Writer        | `Provider::of`   | `Provider::write_of` | `Provider::writer_of` |
//!
//! > **Note**: You can also use `Provider::state_of` to directly access the
//! > concrete state type
//! > instead of using the boxed state accessors (`reader_of`, `watcher_of`,
//! > `writer_of`).

//! ```
//! use ribir_core::prelude::*;
//!
//! let state = Stateful::new(1i32);
//! providers! {
//!   providers: [Provider::writer(state, None)],
//!   @ {
//!     let ctx = BuildCtx::get();
//!
//!     // Access value
//!     assert_eq!(*Provider::of::<i32>(ctx).unwrap(), 1);
//!
//!     // Access state container
//!     let state_ref = Provider::state_of::<Stateful<i32>>(ctx).unwrap();
//!
//!     // Get writer handle
//!     let mut value = Provider::write_of::<i32>(ctx).unwrap();
//!     *value = 42; // Modify state
//!     assert_eq!(*state_ref.read(), 42);
//!
//!     @Text { text: "State management example" }
//!   }
//! };
//! ```
//!
//! ## Scoping Rules
//!
//! Providers are only visible to their descendants. During widget building:
//! 1. Provider scope begins when `Providers` widget is created
//! 2. Scope ends when composition completes
//!
//! ### Correct Usage
//! ```rust
//! use ribir::prelude::*;
//!
//! let good = fn_widget! {
//!   @Row {
//!     // Outside provider scope - NO access
//!     @Text { text: "Can't access provider here" }
//!
//!     @Providers {
//!       providers: [Provider::new(1i32)],
//!       // Inside provider scope - access available
//!       @Text { text: "Can access provider here" }
//!     }
//!   }
//! };
//! ```
//!
//! ### Incorrect Usage
//! ```rust
//! use ribir::prelude::*;
//!
//! let bad = fn_widget! {
//!   let providers = Providers::new([Provider::new(1i32)]);
//!   // ❌ Improper access outside composition scope
//!   let _ = Provider::of::<i32>(BuildCtx::get());
//!
//!   @Row {
//!     @Text { text: "Building..." }
//!     @ (providers) {  // Providers attached late
//!       @Text { text: "Actual provider scope" }
//!     }
//!   }
//! };
//! ```
use std::{cell::RefCell, convert::Infallible};

use ops::box_it::CloneableBoxOp;
use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::prelude::*;

/// The widget that provides data to its descendants. See the
/// [module-level](self) documentation for more details.
pub struct Providers {
  providers: RefCell<SmallVec<[Provider; 1]>>,
}

/// Macro used to generate a function widget using `Providers` as the root
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
pub trait ProviderSetup: Any {
  fn setup(self: Box<Self>, ctx: &mut ProviderCtx) -> Box<dyn ProviderRestore>;
}

/// This trait is used to retrieve the providers from the context. In most
/// cases, you don't need to worry about it unless you want to customize the
/// retrieval process.
pub trait ProviderRestore {
  fn restore(self: Box<Self>, ctx: &mut ProviderCtx) -> Box<dyn ProviderSetup>;
}

/// The context used to store the providers.
#[derive(Default)]
pub struct ProviderCtx {
  data: ahash::AHashMap<TypeInfo, Box<dyn Query>>,
  /// The stack is used to temporarily store the providers that are set up and
  /// will be popped and restored when the scope is exited.
  setup_providers: Vec<(WidgetId, *const Providers)>,
}

impl Provider {
  /// Creates a value provider accessible via [`Provider::of`].
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let w = providers! {
  ///   providers: [Provider::new(1i32)],
  ///   @ {
  ///     assert_eq!(*Provider::of::<i32>(BuildCtx::get()).unwrap(), 1);
  ///     Void
  ///   }
  /// };
  /// ```
  pub fn new<T: 'static>(value: T) -> Provider { Provider::Setup(Box::new(Setup::new(value))) }

  /// Creates a provider for an immutable reader of the given value.
  ///
  /// Clones the reader to prevent writer leaks when establishing the provider.
  ///
  /// Access methods:
  /// - Value reference: [`Provider::of`]
  /// - Concrete reader: [`Provider::state_of`]
  /// - Boxed reader: [`Provider::reader_of`]
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let w = providers! {
  ///   providers: [Provider::reader(Stateful::new(1i32))],
  ///   @ {
  ///     let ctx = BuildCtx::get();
  ///     // Access value
  ///     assert_eq!(*Provider::of::<i32>(ctx).unwrap(), 1);
  ///
  ///     // Access concrete reader
  ///     let reader = Provider::state_of::<Reader<i32>>(ctx);
  ///     assert_eq!(*reader.unwrap().read(), 1);
  ///
  ///     // Access boxed reader
  ///     let boxed_reader = Provider::reader_of::<i32>(ctx);
  ///     assert_eq!(*boxed_reader.unwrap().read(), 1);
  ///     Void
  ///   }
  /// };
  /// ```
  pub fn reader(value: impl StateReader<Value: Sized, Reader: Query>) -> Provider {
    Provider::Setup(Box::new(Setup::from_state(value.clone_reader())))
  }

  /// Creates a provider for a value watcher.
  ///
  /// Clones the watcher to prevent writer leaks when establishing the provider.
  ///
  /// Access methods:
  /// - Value reference: [`Provider::of`]
  /// - Concrete watcher: [`Provider::state_of`]
  /// - Boxed watcher: [`Provider::watcher_of`]
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let w = providers! {
  ///   providers: [Provider::watcher(Stateful::new(1i32))],
  ///   @ {
  ///     let ctx = BuildCtx::get();
  ///     // Access value
  ///     assert_eq!(*Provider::of::<i32>(ctx).unwrap(), 1);
  ///
  ///     // Access concrete watcher
  ///     let watcher = Provider::state_of::<Watcher<Reader<i32>>>(ctx);
  ///     assert_eq!(*watcher.unwrap().read(), 1);
  ///
  ///     // Access boxed watcher
  ///     let boxed_watcher = Provider::watcher_of::<i32>(ctx);
  ///     assert_eq!(*boxed_watcher.unwrap().read(), 1);
  ///     Void
  ///   }
  /// };
  /// ```
  pub fn watcher(value: impl StateWatcher<Value: Sized, Watcher: Query>) -> Provider {
    Provider::Setup(Box::new(Setup::from_state(value.clone_watcher())))
  }

  /// Creates a provider for a mutable state writer.
  ///
  /// Access methods:
  /// - Value reference: [`Provider::of`]
  /// - Mutable reference: [`Provider::write_of`]
  /// - Concrete writer: [`Provider::state_of`]
  /// - Boxed writer: [`Provider::writer_of`]
  ///
  /// # Dirty Phase Handling
  ///
  /// The `dirty` parameter controls update propagation:
  /// - `Some(phase)`: Triggers dirty marking in specified phase
  /// - `None`: Requires manual update notifications
  ///
  /// > Use `Some(DirtyPhase::LayoutSubtree)` when value affects layout/painting
  /// > as providers can impact entire subtrees.
  ///
  /// # Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// let w = providers! {
  ///   providers: [Provider::writer(Stateful::new(1i32), None)],
  ///   @ {
  ///     let ctx = BuildCtx::get();
  ///     // Access value
  ///     assert_eq!(*Provider::of::<i32>(ctx).unwrap(), 1);
  ///
  ///     // Get mutable reference
  ///     let mut_ref = Provider::write_of::<i32>(ctx);
  ///     *mut_ref.unwrap() = 2;
  ///
  ///     // Access concrete writer
  ///     let writer = Provider::state_of::<Stateful<i32>>(ctx);
  ///     assert_eq!(*writer.unwrap().write(), 2);
  ///
  ///     // Access boxed writer
  ///     let boxed_writer = Provider::writer_of::<i32>(ctx);
  ///     *boxed_writer.unwrap().write() = 3;
  ///     Void
  ///   }
  /// };
  /// ```
  pub fn writer<V: 'static>(
    value: impl StateWriter<Value = V> + Query, dirty: Option<DirtyPhase>,
  ) -> Provider {
    if let Some(dirty) = dirty {
      let writer = WriterSetup {
        modifies: value.raw_modifies(),
        info: Provider::info::<V>(),
        value: Box::new(value),
        dirty,
      };
      Provider::Setup(Box::new(writer))
    } else {
      Provider::Setup(Box::new(Setup::from_state(value)))
    }
  }

  /// Gets a shared reference to value `V` from provider context
  ///
  /// **Requires** `V` was provided via:
  /// - [`Provider::new`]
  /// - State reader value ([`Provider::reader`])
  /// - State watcher value ([`Provider::watcher`])
  /// - State writer value ([`Provider::writer`])
  pub fn of<V: 'static>(ctx: &impl AsRef<ProviderCtx>) -> Option<QueryRef<'_, V>> {
    ctx.as_ref().get_provider::<V>()
  }

  /// Gets an exclusive mutable reference to value `V`
  ///
  /// **Requires** `V` was provided via [`Provider::writer`]
  pub fn write_of<V: 'static>(ctx: &impl AsRef<ProviderCtx>) -> Option<WriteRef<'_, V>> {
    ctx.as_ref().get_provider_write::<V>()
  }

  /// Gets the concrete state instance `S`
  ///
  /// **Requires** `S` was provided via:
  /// - [`Provider::reader`]
  /// - [`Provider::writer`]
  /// - [`Provider::watcher`]
  pub fn state_of<S>(ctx: &impl AsRef<ProviderCtx>) -> Option<QueryRef<'_, S>>
  where
    S: StateReader<Value: Sized + 'static>,
  {
    ctx.as_ref().get_provider_state::<S>()
  }

  /// Gets a boxed state writer for value `V`
  ///
  /// **Requires** `V` was provided via [`Provider::writer`]
  pub fn writer_of<V: 'static>(
    ctx: &impl AsRef<ProviderCtx>,
  ) -> Option<Box<dyn StateWriter<Value = V>>> {
    ctx
      .as_ref()
      .get_state_of_value::<Box<dyn StateWriter<Value = V>>>()
      .map(|s| s.clone_writer())
  }

  /// Gets a boxed state watcher for value `V`
  ///
  /// **Requires** `V` was provided via:
  /// - [`Provider::watcher`]
  /// - [`Provider::writer`]
  pub fn watcher_of<V: 'static>(
    ctx: &impl AsRef<ProviderCtx>,
  ) -> Option<Box<dyn StateWatcher<Value = V>>> {
    ctx
      .as_ref()
      .get_state_of_value::<Box<dyn StateWatcher<Value = V>>>()
      .map(|s| s.clone_watcher())
  }

  /// Gets a boxed state reader for value `V`
  ///
  /// **Requires** `V` was provided via:
  /// - [`Provider::reader`]
  /// - [`Provider::watcher`]
  /// - [`Provider::writer`]
  pub fn reader_of<V: 'static>(
    ctx: &impl AsRef<ProviderCtx>,
  ) -> Option<Box<dyn StateReader<Value = V>>> {
    ctx
      .as_ref()
      .get_state_of_value::<Box<dyn StateReader<Value = V>>>()
      .map(|s| s.clone_reader())
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
  pub fn with_providers(&mut self, providers: impl Into<SmallVec<[Provider; 1]>>) -> &mut Self {
    if let Some(vec) = self.providers.as_mut() {
      vec.extend(providers.into());
    } else {
      self.providers = Some(providers.into());
    }
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
    let mut builder = Providers::declarer();
    builder.with_providers(providers);
    builder.finish()
  }

  pub(crate) fn setup_providers(&self, map: &mut ProviderCtx) {
    for p in self.providers.borrow_mut().iter_mut() {
      p.setup(map);
    }
  }

  pub(crate) fn restore_providers(&self, map: &mut ProviderCtx) {
    for p in self.providers.borrow_mut().iter_mut().rev() {
      p.restore(map);
    }
  }
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

  pub(crate) fn get_provider<T: 'static>(&self) -> Option<QueryRef<'_, T>> {
    let info = Provider::info::<T>();
    self
      .data
      .get(&info)
      .and_then(|q| q.query(&QueryId::of::<T>()))
      .and_then(QueryHandle::into_ref)
  }

  pub(crate) fn get_provider_write<T: 'static>(&self) -> Option<WriteRef<'_, T>> {
    let info = Provider::info::<T>();
    self
      .data
      .get(&info)
      .and_then(|q| q.query_write(&QueryId::of::<T>()))
      .and_then(QueryHandle::into_mut)
  }

  pub fn get_provider_state<S>(&self) -> Option<QueryRef<'_, S>>
  where
    S: StateReader<Value: Sized + 'static>,
  {
    self.get_state_of_value::<S>()
  }

  pub(crate) fn get_state_of_value<S: StateReader<Value: Sized + 'static>>(
    &self,
  ) -> Option<QueryRef<'_, S>> {
    let info = Provider::info::<S::Value>();
    self
      .data
      .get(&info)
      .and_then(|q| q.query(&QueryId::of::<S>()))
      .and_then(QueryHandle::into_ref)
  }

  pub(crate) fn remove_key_value_if(
    &mut self, f: impl Fn(&TypeInfo) -> bool,
  ) -> Vec<(TypeInfo, Box<dyn Query>)> {
    let mut out = Vec::new();
    let keys = self.data.keys().cloned().collect::<Vec<_>>();
    for k in keys {
      if f(&k)
        && let Some(v) = self.data.remove(&k)
      {
        out.push((k, v));
      }
    }
    out
  }
}

impl Providers {
  pub fn with_child<'w, K>(self, child: impl IntoWidget<'w, K>) -> Widget<'w> {
    let mut child = child.into_widget();
    self.restore_providers(BuildCtx::get_mut().as_mut());

    for provider in self.providers.borrow_mut().iter_mut() {
      let Provider::Setup(p) = provider else { unreachable!() };
      let p_any: &mut dyn Any = &mut **p;
      if let Some(writer) = p_any.downcast_mut::<WriterSetup>() {
        child = child.dirty_on(writer.modifies.clone(), writer.dirty);
      }
    }

    Widget::from_fn(move |ctx| {
      self.setup_providers(ctx.as_mut());
      let id = ctx.build(child);
      self.restore_providers(ctx.as_mut());
      id.wrap_node(ctx.tree_mut(), |render| Box::new(ProvidersRender { providers: self, render }));
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
  providers: Providers,
  render: Box<dyn RenderQueryable>,
}

impl Query for ProvidersRender {
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all(query_id, out);
    if query_id == &QueryId::of::<Providers>() {
      out.push(QueryHandle::new(&self.providers));
    }
  }

  fn query_all_write<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all_write(query_id, out);
  }

  fn query<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    if query_id == &QueryId::of::<Providers>() {
      Some(QueryHandle::new(&self.providers))
    } else {
      self.render.query(query_id)
    }
  }

  fn query_write<'q>(&'q self, query_id: &QueryId) -> Option<QueryHandle<'q>> {
    self.render.query_write(query_id)
  }

  fn queryable(&self) -> bool { true }
}

impl Render for ProvidersRender {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let Self { render, providers } = self;
    providers.setup_providers(ctx.as_mut());
    let size = render.perform_layout(clamp, ctx);
    providers.restore_providers(ctx.as_mut());
    size
  }

  fn visual_box(&self, ctx: &mut VisualCtx) -> Option<Rect> { self.render.visual_box(ctx) }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let Self { render, providers } = self;
    let id = ctx.id();
    // The providers will be popped in the `PaintingCtx::finish` method, once the
    // painting of the entire subtree is completed.
    ctx.as_mut().push_providers(id, providers);

    render.paint(ctx);
  }

  fn hit_test(&self, ctx: &mut HitTestCtx, pos: Point) -> HitTest {
    let Self { render, providers } = self;
    let id = ctx.id();
    // The providers will be popped in the `HitTestCtx::finish` method, once the
    // hit test of the entire subtree is completed.
    ctx.as_mut().push_providers(id, providers);
    render.hit_test(ctx, pos)
  }

  fn size_affected_by_child(&self) -> bool { self.render.size_affected_by_child() }

  fn get_transform(&self) -> Option<Transform> { self.render.get_transform() }
}

pub(crate) struct Setup {
  info: TypeInfo,
  value: Box<dyn Query>,
}

struct Restore {
  info: TypeInfo,
  value: Option<Box<dyn Query>>,
}

struct WriterSetup {
  info: TypeInfo,
  value: Box<dyn Query>,
  modifies: CloneableBoxOp<'static, ModifyInfo, Infallible>,
  dirty: DirtyPhase,
}

struct WriterRestore {
  info: TypeInfo,
  restore_value: Option<Box<dyn Query>>,
  modifies: CloneableBoxOp<'static, ModifyInfo, Infallible>,
  dirty: DirtyPhase,
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
    let v = restore(info, value, map);
    Box::new(Setup { info, value: v })
  }
}

impl ProviderSetup for WriterSetup {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let WriterSetup { info, value, modifies, dirty } = *self;
    let old = map.set_raw_provider(info, value);
    Box::new(WriterRestore { info, restore_value: old, modifies, dirty })
  }
}

impl ProviderRestore for WriterRestore {
  fn restore(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderSetup> {
    let WriterRestore { info, restore_value, modifies, dirty } = *self;
    let value = restore(info, restore_value, map);
    Box::new(WriterSetup { info, value, modifies, dirty })
  }
}

fn restore(
  info: TypeInfo, restore_value: Option<Box<dyn Query>>, map: &mut ProviderCtx,
) -> Box<dyn Query> {
  let v = if let Some(v) = restore_value {
    map.set_raw_provider(info, v)
  } else {
    map.remove_raw_provider(&info)
  };
  if let Some(v) = v {
    v
  } else {
    panic!("Provider restore not matched");
  }
}

impl Setup {
  pub(crate) fn new<T: 'static>(value: T) -> Self {
    Setup { info: Provider::info::<T>(), value: Box::new(Queryable(value)) }
  }

  pub(crate) fn custom(info: TypeInfo, value: Box<dyn Query>) -> Self { Setup { info, value } }

  pub(crate) fn from_state<V: 'static>(value: impl StateReader<Value = V> + Query) -> Self {
    Setup { info: Provider::info::<V>(), value: Box::new(value) }
  }
}

impl AsRef<ProviderCtx> for ProviderCtx {
  fn as_ref(&self) -> &ProviderCtx { self }
}

impl AsMut<ProviderCtx> for ProviderCtx {
  fn as_mut(&mut self) -> &mut ProviderCtx { self }
}

#[cfg(test)]
mod tests {

  use std::cell::Cell;

  use smallvec::smallvec;

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[test]
  fn smoke() {
    reset_test_env!();

    let wnd = TestWindow::from_widget(mock_multi! {
      @Providers {
        providers: smallvec![Provider::new(Color::RED)],
        @ {
          assert_eq!(BuildCtx::color().clone_value(), Color::RED);
          @MockMulti {
            @fn_widget!{
              assert_eq!(BuildCtx::color().clone_value(), Color::RED);
              Void
            }
          }
        }
      }
      @ {
        let color = BuildCtx::color();
        assert_eq!(color.clone_value(), Palette::of(BuildCtx::get()).primary());
        Void
      }
    });
    wnd.draw_frame();
  }

  #[test]
  fn embedded() {
    reset_test_env!();

    let wnd = TestWindow::from_widget(providers! {
      providers: smallvec![Provider::new(Color::RED)],
      @Providers {
        providers: smallvec![ContainerColor::provider(Color::GREEN)],
        @ {
          let container_color = BuildCtx::container_color();
          assert_eq!(container_color.clone_value(), Color::GREEN);
          let color = BuildCtx::color();
          assert_eq!(color.clone_value(), Color::RED);
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

    let wnd = TestWindow::from_widget(providers! {
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
      providers: [Provider::new(1i32)],
      @MockBox {
        size: Size::new(1.,1.),
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          *$write(w_value) = *v;
          Void
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
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
        providers: [Provider::new(1i32)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          *$write(w_value1) = *v;
          Void
        }
      }

      @Providers {
        providers: smallvec![Provider::new(2i32)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          *$write(w_value2) = *v;
          Void
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
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
      providers: [Provider::new(w_value.clone_writer())],
      @ {
        // We do not allow the use of the build context in the pipe at the moment.
        let value = Provider::of::<Stateful<i32>>(BuildCtx::get())
          .unwrap().clone_writer();
        pipe!(*$read(trigger)).map(move |_| {
          *$write(value) += 1;
          @Void {}
        })
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    assert_eq!(*value.read(), 1);

    *w_trigger.write() = false;
    wnd.draw_frame();
    assert_eq!(*value.read(), 2);
  }

  #[test]
  fn dirty_paint() {
    reset_test_env!();

    struct PaintCnt {
      layout_cnt: Cell<usize>,
      paint_cnt: Cell<usize>,
    }

    impl Render for PaintCnt {
      fn perform_layout(&self, clamp: BoxClamp, _: &mut LayoutCtx) -> Size {
        self.layout_cnt.set(self.layout_cnt.get() + 1);
        clamp.max
      }
      fn paint(&self, _: &mut PaintingCtx) { self.paint_cnt.set(self.paint_cnt.get() + 1); }
    }

    let paint = Stateful::new(true);
    let c_paint = paint.clone_writer();
    let layout = Stateful::new(());
    let c_layout = layout.clone_writer();

    let (cnt, w_cnt) = split_value(PaintCnt { layout_cnt: Cell::new(0), paint_cnt: Cell::new(0) });

    let w = providers! {
      providers: smallvec![
        Provider::writer(paint.clone_writer(), Some(DirtyPhase::Paint)),
        Provider::writer(layout.clone_writer(), Some(DirtyPhase::LayoutSubtree)),
      ],
      @ {w_cnt.clone_writer()}
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    assert_eq!(cnt.read().layout_cnt.get(), 1);
    assert_eq!(cnt.read().paint_cnt.get(), 1);

    {
      let _ = &mut *c_paint.write();
    }

    wnd.draw_frame();
    assert_eq!(cnt.read().layout_cnt.get(), 1);
    assert_eq!(cnt.read().paint_cnt.get(), 2);

    {
      let _ = &mut *c_layout.write();
    }

    wnd.draw_frame();
    assert_eq!(cnt.read().layout_cnt.get(), 2);
    assert_eq!(cnt.read().paint_cnt.get(), 3);
  }

  #[test]
  fn boxed_reader() {
    reset_test_env!();

    let w = fn_widget! {
      let boxed_reader: Box<dyn StateReader<Value = i32>> = Box::new(Stateful::new(1i32));

      @Providers {
        providers: [Provider::reader(boxed_reader)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          assert_eq!(*v, 1);

          let v = Provider::state_of::<Box<dyn StateReader<Value = i32>>>(BuildCtx::get()).unwrap();
          assert_eq!(*v.read(), 1);
          Void
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
  }

  #[test]
  fn boxed_watcher() {
    reset_test_env!();

    let w = fn_widget! {
      let boxed_watcher: Box<dyn StateWatcher<Value = i32>> = Box::new(Stateful::new(1i32));

      @Providers {
        providers: [Provider::watcher(boxed_watcher)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          assert_eq!(*v, 1);

          let v = Provider::state_of::<Box<dyn StateWatcher<Value = i32>>>(
              BuildCtx::get()
            )
            .unwrap();
          assert_eq!(*v.read(), 1);
          Void
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
  }

  #[test]
  fn boxed_writer() {
    reset_test_env!();

    let w = fn_widget! {
      let boxed_writer: Box<dyn StateWriter<Value = i32>> = Box::new(Stateful::new(1i32));

      @Providers {
        providers: [Provider::writer(boxed_writer, None)],
        @ {
          let v = Provider::of::<i32>(BuildCtx::get()).unwrap();
          assert_eq!(*v, 1);

          let v = Provider::state_of::<Box<dyn StateWriter<Value = i32>>>(
              BuildCtx::get()
            )
            .unwrap();
          assert_eq!(*v.read(), 1);
          Void
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
  }
}
