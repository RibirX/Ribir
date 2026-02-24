//! `Class` is a widget used to specify a class for its child widget. It draws
//! inspiration from the HTML class attribute, enabling the sharing of
//! consistent styles across multiple elements with different functionalities.
//!
//! In Ribir, you can define a class name using `class_name!`, which the `Class`
//! widget can utilize to locate a function in `Theme` or `Classes` to transform
//! its child into another widget. This means that `Class` not only facilitates
//! style sharing but also allows for the addition of action behavior or
//! composition with multiple widgets.
//!
//! # Example
//!
//! ```no_run
//! use ribir::prelude::*;
//!
//! class_names!(RED_BORDER);
//!
//! let theme_fn = move || {
//!   let mut theme = Theme::default();
//!   // Define how `RED_BORDER` transforms a widget.
//!   theme.classes.insert(
//!     RED_BORDER,
//!     style_class! {
//!       border: Border::all(BorderSide::new(2., Color::RED.into()))
//!     },
//!   );
//!   theme
//! };
//!
//! let w = fn_widget! {
//!   @Container {
//!     size: Size::new(100., 100.),
//!     class: RED_BORDER,
//!   }
//! };
//!
//! App::run(w).with_app_theme(theme_fn);
//! ```

use std::{cell::RefCell, convert::Infallible, hash::Hash, rc::Rc};

use pipe::PipeNode;
use rxrust::{
  observable::boxed::LocalBoxedObservable,
  subscription::{BoxedSubscription, IntoBoxedSubscription, SubscriptionGuard},
};
use smallvec::{SmallVec, smallvec};

use crate::{pipe::GenRange, prelude::*, ticker::FrameMsg, window::WindowId};

/// A collection of class implementations that are part of the `Theme`.
#[derive(Default, Clone)]
pub struct Classes {
  pub(crate) store: ahash::HashMap<ClassName, ClassImpl>,
}

/// The macro is used to create a class implementation by accepting declarations
/// of the built-in widget fields.
#[macro_export]
macro_rules! style_class {
  ($($field: ident: $value: expr),* $(,)?) => {
    (move |widget: $crate::prelude::Widget| {
      rdl! {
        @FatObj {
          $( $field: $value, )*
          @ { widget }
        }
      }.into_widget()
    }) as $crate::prelude::ClassImpl
  };
}

/// This macro generates a function for creating a styled widget with predefined
/// fields. It simplifies the process of applying consistent styles to widgets.
///
/// # Usage
/// ```rust
/// use ribir::prelude::*;
///
/// named_style_impl!(primary_button => {
///   padding: EdgeInsets::all(8.),
///   background: Color::BLUE,
/// });
/// ```
#[macro_export]
macro_rules! named_style_impl {
  ($(#[$meta:meta])* $style_name:ident => {
      $($field:ident: $value:expr),* $(,)?
  }) => {
    $(#[$meta])*
    fn $style_name(widget: $crate::prelude::Widget) -> $crate::prelude::Widget {
      rdl! {
        @FatObj {
          $( $field: $value, )*
          @ { widget }
        }.into_widget()
      }
    }
  };
}

/// This macro generates multiple styled widget builder functions in one go.
/// It helps in defining several styles simultaneously, reducing repetition.
///
/// # Example
/// ```
/// use ribir::prelude::*;
///
/// named_styles_impl! {
///   /// Secondary button style for auxiliary actions
///   secondary_button => {
///       padding: EdgeInsets::all(6.),
///       background: Color::GRAY,
///   },
///
///   /// Danger button style for destructive operations
///   danger_button => {
///       padding: EdgeInsets::all(8.),
///       background: Color::RED,
///   }
/// }
/// ```
#[macro_export]
macro_rules! named_styles_impl {
  ($( $(#[$meta:meta])* $name:ident => { $($field:ident: $value:expr),* $(,)? } ),* $(,)? ) => {
    $(
      named_style_impl! {
        $(#[$meta])*
        $name => { $($field: $value),* }
      }
    )*
  };
}

/// Combines multiple class implementations into a single implementation.
/// This macro takes a list of class implementations and returns a closure
/// that applies each implementation sequentially to a `Widget`.
///
/// The first implementation in the list runs first, and the last one is
/// applied last (closest to the widget). Therefore, the last implementation
/// has the highest visual priority (e.g. `[BASE, SELECTED]` ->
/// `BASE(SELECTED(child))`).
///
/// # Example
/// ```
/// use ribir::prelude::*;
///
/// class_names!(PADDING_AND_BG);
///
/// fn init_classes(classes: &mut Classes) {
///   classes.insert(
///     PADDING_AND_BG,
///     class_chain_impl![
///       style_class! { padding: EdgeInsets::all(4.) },
///       style_class! { background: Color::BLUE }
///     ],
///   );
/// }
/// ```
#[macro_export]
macro_rules! class_chain_impl {
  ($($class: expr),*) => {
    move |mut w: Widget| {
      $(w = $class(w);)*
      w
    }
  };
}

/// A empty class implementation that returns the input widget as is.
pub fn empty_cls(w: Widget) -> Widget { w }

/// This type is utilized to define a constant variable as the name of a
/// `Class`. It can also override its implementation across the `Theme` and
/// `Classes`.
///
/// # Example
///
/// Use `class_names!` to define your class names.
///
/// ```
/// use ribir::prelude::*;
///
/// class_names!(A, B, C);
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ClassName(&'static str);

/// A function that transforms a `Widget` into another `Widget` as a class
/// implementation.
///
/// The function accepts a `Widget` as input and returns a `Widget` as output,
/// ensuring that the input widget is retained in the returned widget.
/// Otherwise, switching the class to another class will fail.
// Note: The provider of `Class` can be tricky, so changing the definition of
// ClassImpl without being careful may result in compatibility issues with the
// previous version's binary. See `[`ClassName::type_info`]`.
pub type ClassImpl = fn(Widget) -> Widget;

#[derive(Default)]
pub struct ClassList {
  classes: SmallVec<[PipeValue<Option<ClassName>>; 1]>,
}

impl ClassList {
  #[inline]
  pub fn new() -> Self { Self::default() }

  #[inline]
  pub fn push<K: ?Sized>(&mut self, class: impl RInto<PipeValue<Option<ClassName>>, K>) {
    self.classes.push(class.r_into());
  }

  #[inline]
  pub fn pop(&mut self) -> Option<PipeValue<Option<ClassName>>> { self.classes.pop() }

  #[inline]
  pub fn len(&self) -> usize { self.classes.len() }

  #[inline]
  pub fn is_empty(&self) -> bool { self.classes.is_empty() }

  #[inline]
  pub fn iter(&self) -> std::slice::Iter<'_, PipeValue<Option<ClassName>>> { self.classes.iter() }

  #[inline]
  pub(crate) fn current(&self) -> ClassSnapshot {
    self
      .classes
      .iter()
      .map(|class| match class {
        PipeValue::Value(v) => *v,
        PipeValue::Pipe { init_value, .. } => *init_value,
      })
      .collect()
  }

  pub(crate) fn into_runtime_parts(self) -> (ClassSnapshot, ClassItemStreams) {
    let mut snapshot = SmallVec::new();
    let mut streams = SmallVec::new();
    for (idx, v) in self.classes.into_iter().enumerate() {
      match v {
        PipeValue::Value(v) => snapshot.push(v),
        PipeValue::Pipe { init_value, pipe } => {
          snapshot.push(init_value);
          let stream = pipe
            .with_effect(ModifyEffect::FRAMEWORK)
            .into_observable();
          streams.push((idx, stream));
        }
      }
    }
    (snapshot, streams)
  }

  pub(crate) fn from_snapshot(snapshot: ClassSnapshot) -> Self {
    let classes = snapshot
      .into_iter()
      .map(PipeValue::Value)
      .collect();
    Self { classes }
  }
}

impl PartialEq for ClassList {
  fn eq(&self, other: &Self) -> bool { self.current() == other.current() }
}

impl Clone for ClassList {
  fn clone(&self) -> Self { Self::from_snapshot(self.current()) }
}

impl IntoIterator for ClassList {
  type Item = PipeValue<Option<ClassName>>;
  type IntoIter = smallvec::IntoIter<[PipeValue<Option<ClassName>>; 1]>;

  #[inline]
  fn into_iter(self) -> Self::IntoIter { self.classes.into_iter() }
}

impl<'a> IntoIterator for &'a ClassList {
  type Item = &'a PipeValue<Option<ClassName>>;
  type IntoIter = std::slice::Iter<'a, PipeValue<Option<ClassName>>>;

  #[inline]
  fn into_iter(self) -> Self::IntoIter { self.classes.iter() }
}

impl FromIterator<ClassName> for ClassList {
  fn from_iter<T: IntoIterator<Item = ClassName>>(iter: T) -> Self {
    Self {
      classes: iter
        .into_iter()
        .map(|v| PipeValue::Value(Some(v)))
        .collect(),
    }
  }
}

impl FromIterator<PipeValue<Option<ClassName>>> for ClassList {
  fn from_iter<T: IntoIterator<Item = PipeValue<Option<ClassName>>>>(iter: T) -> Self {
    Self { classes: iter.into_iter().collect() }
  }
}

/// This widget is used to apply class to its child widget by the `ClassName`.
#[derive(Default)]
pub struct Class {
  pub class: ClassList,
}

/// This macro is used to generate a function widget using `Class` as the root
/// widget.
#[macro_export]
macro_rules! class {
  ($($t: tt)*) => { fn_widget! { @Class { $($t)* } } };
}

#[macro_export]
macro_rules! class_list {
  ($($class:expr),* $(,)?) => {{
    let mut list = $crate::prelude::ClassList::new();
    $(
      let class_item: $crate::prelude::PipeValue<Option<$crate::prelude::ClassName>> =
        $crate::prelude::RInto::r_into($class);
      list.push(class_item);
    )*
    list
  }};
}

/// This macro is utilized to define class names; ensure that your name is
/// unique within the application.
#[macro_export]
macro_rules! class_names {
  ($(
    $(#[$outer:meta])?
    $name:ident
  ),* $(,)?) => {
    $(
      $(#[$outer])?
      pub const $name: ClassName = ClassName::new(stringify!($name));
    )*
  };
}

impl ClassName {
  pub const fn new(name: &'static str) -> Self { ClassName(name) }

  fn type_info(&self) -> TypeInfo {
    const LAYOUT: std::alloc::Layout = std::alloc::Layout::new::<ClassImpl>();
    // Tricky: We disregard the package version since the type remains stable.
    // Instead, we include the class name in the type information, allowing each
    // unique class name to serve as a distinct provider.
    TypeInfo { name: std::any::type_name::<ClassName>(), pkg_version: self.0, layout: &LAYOUT }
  }

  fn from_info(info: &TypeInfo) -> Option<Self> {
    if info.name == std::any::type_name::<ClassName>() {
      Some(Self(info.pkg_version))
    } else {
      None
    }
  }
}

impl Classes {
  #[inline]
  /// Assigns the implementation of `cls` to the store and returns the previous
  /// implementation, if any.
  ///
  /// Note: You must ensure that the widget provided in the `ClassImpl` is
  /// maintained in the returned widget of the `ClassImpl`.
  pub fn insert(&mut self, cls: ClassName, f: ClassImpl) -> Option<ClassImpl> {
    self.store.insert(cls, f)
  }

  pub(crate) fn reader_into_provider<R: StateReader<Value = Classes> + Query>(this: R) -> Provider {
    Provider::Setup(Box::new(ClassesReaderSetup(this)))
  }

  fn take_overridden_from(&self, map: &mut ProviderCtx) -> Vec<(TypeInfo, Box<dyn Query>)> {
    map.remove_key_value_if(|info| {
      ClassName::from_info(info).is_some_and(|name| self.store.contains_key(&name))
    })
  }
}

struct ClassesReaderSetup<T>(T);

struct ClassesRestore {
  overrides: Vec<(TypeInfo, Box<dyn Query>)>,
  classes: Box<dyn ProviderRestore>,
}

impl ProviderSetup for Classes {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let overrides = self.take_overridden_from(map);
    let classes = Box::new(Setup::new(*self)).setup(map);
    Box::new(ClassesRestore { overrides, classes })
  }
}

impl<R: StateReader<Value = Classes> + Query> ProviderSetup for ClassesReaderSetup<R> {
  fn setup(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderRestore> {
    let classes = self.0;
    let overrides = classes.read().take_overridden_from(map);
    let classes = Box::new(Setup::from_state(classes)).setup(map);
    Box::new(ClassesRestore { overrides, classes })
  }
}

impl ProviderRestore for ClassesRestore {
  fn restore(self: Box<Self>, map: &mut ProviderCtx) -> Box<dyn ProviderSetup> {
    let setup = self.classes.restore(map);
    for (info, provider) in self.overrides {
      let _old = map.set_raw_provider(info, provider);
      debug_assert!(_old.is_none());
    }
    setup
  }
}

impl Declare for Class {
  type Builder = FatObj<()>;
  #[inline]
  fn declarer() -> Self::Builder { FatObj::new(()) }
}

impl<'c> ComposeChild<'c> for Class {
  type Child = Widget<'c>;

  fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'c> {
    let f = move || match this.try_into_value() {
      Ok(c) => compose_from_value(c, child),
      Err(writer) => compose_from_writer(writer, child),
    };
    FnWidget::new(f).into_widget()
  }
}

fn compose_from_value<'w>(class: Class, child: Widget<'w>) -> Widget<'w> {
  let (snapshot, streams) = class.class.into_runtime_parts();
  if streams.is_empty() {
    compose_with_classes(snapshot, child)
  } else {
    let ctx = BuildCtx::get();
    let runtime = Rc::new(NestedClassRuntime::new(ctx.window().id(), ctx.tree().dummy_id()));
    runtime
      .build_layers(
        snapshot,
        &streams
          .iter()
          .map(|(i, _)| *i)
          .collect::<Vec<_>>(),
        child,
      )
      .on_build(move |child_id| {
        attach_runtime_and_rebind(child_id, runtime.clone(), streams);
      })
  }
}

fn compose_from_writer<'w>(
  writer: impl StateWriter<Value = Class>, child: Widget<'w>,
) -> Widget<'w> {
  let ctx = BuildCtx::get();
  let runtime = Rc::new(NestedClassRuntime::new(ctx.window().id(), ctx.tree().dummy_id()));
  let (current, streams) = Class::extract_runtime_parts_and_freeze(&writer);
  let manager = LayerManager::new(ctx.window().id(), ctx.tree().dummy_id(), current);
  let child = manager.wrap_child(child);
  let runtime_build = runtime.clone();

  let dynamic_indices: Vec<usize> = streams.iter().map(|(i, _)| *i).collect();
  manager
    .build_initial(child, &move |classes, child| {
      runtime_build.build_layers(classes, &dynamic_indices, child)
    })
    .on_build(move |child_id| {
      attach_runtime_and_rebind(child_id, runtime.clone(), streams);
      attach_outer_stream_subscription(child_id, writer, runtime, manager);
    })
}

fn attach_runtime_and_rebind(
  child_id: WidgetId, runtime: Rc<NestedClassRuntime>, streams: ClassItemStreams,
) {
  child_id.attach_anonymous_data(runtime.clone(), BuildCtx::get_mut().tree_mut());
  runtime.rebind_dynamic_streams(streams);
}

fn attach_outer_stream_subscription(
  child_id: WidgetId, writer: impl StateWriter<Value = Class>, runtime: Rc<NestedClassRuntime>,
  manager: LayerManager<ClassSnapshot>,
) {
  let stream_writer = writer.clone_writer();
  let callback_writer = writer.clone_writer();
  let stream = pipe!($read(stream_writer).class.current())
    .with_effect(ModifyEffect::FRAMEWORK)
    .into_observable();
  let sampler = BuildCtx::get()
    .window()
    .frame_tick_stream()
    .filter(|msg| matches!(msg, FrameMsg::NewFrame(_)));
  let u = stream
    .sample(sampler)
    .subscribe(move |_| {
      runtime.refresh_from_state(&callback_writer, &manager);
    })
    .unsubscribe_when_dropped();
  child_id.attach_anonymous_data(u, BuildCtx::get_mut().tree_mut());
}

/// Compose a widget with multiple classes, chaining them in reverse order.
fn compose_with_classes(classes: ClassSnapshot, child: Widget) -> Widget {
  let mut widget = child;
  for cls in classes.into_iter().rev() {
    widget = apply_class(cls, widget);
  }
  widget
}

fn apply_class(class: Option<ClassName>, w: Widget) -> Widget {
  if let Some(cls_impl) = class_impl(class) { cls_impl(w) } else { w }
}

/// Resolve the class implementation for a given class name.
/// Override providers (set via `Class::provider`) take precedence over the
/// `Classes` store in the theme.
fn class_impl(class: Option<ClassName>) -> Option<ClassImpl> {
  let cls = class?;
  let ctx = BuildCtx::get();
  let override_cls = ctx
    .as_ref()
    .get_raw_provider(&cls.type_info())
    .and_then(|q| q.query(&QueryId::of::<ClassImpl>()))
    .and_then(QueryHandle::into_ref::<ClassImpl>)
    .map(|i| *i);

  override_cls.or_else(|| {
    Provider::of::<Classes>(ctx)?
      .store
      .get(&cls)
      .copied()
  })
}

impl Class {
  /// Creating a provider for a class, which can either provide the
  /// implementation of a class or be used to override the implementation of a
  /// class.
  ///
  /// This is a builtin field of FatObj. You can simply set the `class` field
  /// to attach a Class widget to the host widget.
  ///
  /// ## Example
  ///
  /// ```
  /// use ribir_core::prelude::*;
  ///
  /// class_names!(RED_BOX);
  /// let w = providers! {
  ///   providers: [
  ///     Class::provider(RED_BOX, style_class!{
  ///       background: Color::RED,
  ///       clamp: BoxClamp::fixed_size(Size::splat(48.))
  ///     }),
  ///   ],
  ///   @Void { class: RED_BOX }
  /// };
  /// ```
  pub fn provider(name: ClassName, cls_impl: ClassImpl) -> Provider {
    let setup = Setup::custom(name.type_info(), Box::new(Queryable(cls_impl)));
    Provider::Setup(Box::new(setup))
  }

  fn extract_runtime_parts_and_freeze(
    writer: &impl StateWriter<Value = Self>,
  ) -> (ClassSnapshot, ClassItemStreams) {
    let mut class = writer.silent();
    let list = std::mem::take(&mut class.class);
    let (snapshot, streams) = list.into_runtime_parts();
    class.class = ClassList::from_snapshot(snapshot.clone());
    (snapshot, streams)
  }
}

type ClassSnapshot = SmallVec<[Option<ClassName>; 1]>;
type ClassItemStreams =
  SmallVec<[(usize, LocalBoxedObservable<'static, Option<ClassName>, Infallible>); 1]>;
type LayerManagers = SmallVec<[Option<LayerManager<Option<ClassName>>>; 1]>;

struct NestedClassRuntime {
  wnd_id: WindowId,
  dummy_id: WidgetId,
  managers: RefCell<LayerManagers>,
  subscriptions: RefCell<SmallVec<[SubscriptionGuard<BoxedSubscription>; 1]>>,
}

impl NestedClassRuntime {
  fn new(wnd_id: WindowId, dummy_id: WidgetId) -> Self {
    Self {
      wnd_id,
      dummy_id,
      managers: RefCell::new(SmallVec::new()),
      subscriptions: RefCell::new(SmallVec::new()),
    }
  }

  fn build_layers<'w>(
    &self, classes: ClassSnapshot, dynamic_indices: &[usize], child: Widget<'w>,
  ) -> Widget<'w> {
    let mut widget = child;
    let mut managers = SmallVec::new();
    // Iterate in reverse so the last class wraps closest to the child (innermost).
    // Managers are pushed in reverse iteration order, then reversed so that
    // managers[i] corresponds to classes[i].
    for (idx, cls) in classes.into_iter().enumerate().rev() {
      if dynamic_indices.contains(&idx) {
        let manager = LayerManager::new(self.wnd_id, self.dummy_id, cls);
        widget = manager.wrap_child(widget);
        widget = manager.build_initial(widget, &apply_class);
        managers.push(Some(manager));
      } else {
        widget = apply_class(cls, widget);
        managers.push(None);
      }
    }
    managers.reverse();
    *self.managers.borrow_mut() = managers;
    widget
  }

  fn refresh_from_state(
    &self, writer: &impl StateWriter<Value = Class>, manager: &LayerManager<ClassSnapshot>,
  ) {
    let (snapshot, streams) = Class::extract_runtime_parts_and_freeze(writer);
    let dynamic_indices: Vec<usize> = streams.iter().map(|(i, _)| *i).collect();
    manager.update(snapshot, &|classes, child| self.build_layers(classes, &dynamic_indices, child));
    self.rebind_dynamic_streams(streams);
  }

  fn rebind_dynamic_streams(&self, streams: ClassItemStreams) {
    let mut subscriptions = self.subscriptions.borrow_mut();
    subscriptions.clear();

    let managers = self.managers.borrow();
    for (idx, stream) in streams {
      if let Some(Some(manager)) = managers.get(idx) {
        subscriptions.push(manager.mount_stream(stream, apply_class));
      }
    }
  }
}

#[derive(Clone)]
struct LayerManager<T> {
  outer: PipeNode,
  inner: PipeNode,
  wnd_id: WindowId,
  prev: Rc<RefCell<T>>,
}

impl<T> LayerManager<T>
where
  T: Clone + PartialEq + 'static,
{
  fn new(wnd_id: WindowId, dummy_id: WidgetId, prev: T) -> Self {
    let dummy = GenRange::Single(dummy_id);
    Self {
      outer: PipeNode::empty_node(dummy.clone()),
      inner: PipeNode::empty_node(dummy),
      wnd_id,
      prev: Rc::new(RefCell::new(prev)),
    }
  }

  fn wrap_child<'w>(&self, child: Widget<'w>) -> Widget<'w> {
    let inner = self.inner.clone();
    child.on_build(move |inner_id| inner.init_for_single(inner_id))
  }

  fn build_initial<'w>(
    &self, child: Widget<'w>, apply: &impl Fn(T, Widget) -> Widget,
  ) -> Widget<'w> {
    let outer = self.outer.clone();
    apply(self.prev.borrow().clone(), child)
      .on_build(move |outer_id| outer.init_for_single(outer_id))
  }

  fn mount_stream(
    &self, stream: LocalBoxedObservable<'static, T, Infallible>, apply: fn(T, Widget) -> Widget,
  ) -> SubscriptionGuard<BoxedSubscription> {
    let wnd = AppCtx::get_window(self.wnd_id)
      .expect("This handle is not valid because the window is closed");
    let sampler = wnd
      .frame_tick_stream()
      .filter(|msg| matches!(msg, FrameMsg::NewFrame(_)));
    let manager = self.clone();
    stream
      .sample(sampler)
      .subscribe(move |current| {
        manager.update(current, &apply);
      })
      .into_boxed()
      .unsubscribe_when_dropped()
  }

  fn update(&self, next: T, apply: &impl Fn(T, Widget) -> Widget) -> bool {
    if *self.prev.borrow() == next {
      return false;
    }

    let wnd = AppCtx::get_window(self.wnd_id)
      .expect("This handle is not valid because the window is closed");

    let old_host = self.outer.dyn_info().host_id();
    let inner_host = self.inner.dyn_info().host_id();
    if old_host.is_dropped(wnd.tree()) {
      return false;
    }

    let holder = old_host.place_holder(wnd.tree_mut());

    let original_render = self.outer.take_data();
    let _guard = BuildCtx::init_for(old_host, wnd.tree);
    let ctx = BuildCtx::get_mut();

    let pipe_wrapper =
      std::mem::replace(old_host.get_node_mut(ctx.tree_mut()).unwrap(), original_render);

    *inner_host.get_node_mut(ctx.tree_mut()).unwrap() = Box::new(self.inner.clone());
    let next_prev = next.clone();
    let new_id = ctx.build(apply(next, Widget::from_id(inner_host)));

    let tree = ctx.tree_mut();

    if old_host != new_id {
      pipe_wrapper.update_track_id(new_id);
    }

    new_id.wrap_node(tree, |render| {
      self.outer.replace_data(render);
      pipe_wrapper
    });

    if new_id != old_host {
      new_id
        .query_all_iter::<PipeNode>(tree)
        .for_each(|node| node.dyn_info_mut().replace(old_host, new_id));
      holder.replace(new_id, tree);
    }

    if inner_host != old_host {
      old_host.dispose_subtree(tree);
    }

    let mut stack: SmallVec<[WidgetId; 1]> = smallvec![new_id];
    while let Some(w) = stack.pop() {
      // Skip the preserved inner subtree — it is not newly created.
      if w != old_host {
        w.on_mounted_subtree(tree);
        stack.extend(w.children(tree).rev());
      }
    }

    self.outer.dyn_info_mut().gen_range = GenRange::Single(new_id);
    let marker = tree.dirty_marker();
    marker.mark(new_id, DirtyPhase::Layout);
    if new_id != inner_host && new_id.ancestor_of(inner_host, tree) {
      marker.mark(inner_host, DirtyPhase::Layout);
    }

    *self.prev.borrow_mut() = next_prev;
    true
  }
}

impl From<ClassName> for ClassList {
  #[inline]
  fn from(v: ClassName) -> Self { ClassList { classes: smallvec![PipeValue::Value(Some(v))] } }
}

impl From<Option<ClassName>> for ClassList {
  #[inline]
  fn from(v: Option<ClassName>) -> Self { ClassList { classes: smallvec![PipeValue::Value(v)] } }
}

impl From<PipeValue<Option<ClassName>>> for ClassList {
  #[inline]
  fn from(v: PipeValue<Option<ClassName>>) -> Self { ClassList { classes: smallvec![v] } }
}

impl<const N: usize> From<[ClassName; N]> for ClassList {
  #[inline]
  fn from(v: [ClassName; N]) -> Self { ClassList::from_iter(v) }
}

#[cfg(test)]
mod tests {

  use super::*;
  use crate::{reset_test_env, test_helper::*};
  class_names!(MARGIN, BOX_200, CLAMP_50, EMPTY);
  use smallvec::smallvec;

  fn initd_classes() -> Classes {
    let mut classes = Classes::default();
    classes.insert(MARGIN, style_class!(margin: EdgeInsets::all(10.)));
    classes.insert(BOX_200, |w| {
      fn_widget! {
        @MockBox {
          size: Size::new(200., 200.),
          on_mounted: |_| {println!("mounted");},
          @ { w }
        }
      }
      .into_widget()
    });
    classes.insert(
      CLAMP_50,
      style_class! {
        clamp: BoxClamp::fixed_size(Size::new(50., 50.))
      },
    );
    classes
  }

  impl Classes {
    fn into_provider(self) -> Provider { Provider::new(self) }
  }

  #[test]
  fn switch_class() {
    reset_test_env!();

    let (cls, w_cls) = split_value(MARGIN);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(120.));

    *w_cls.write() = BOX_200;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(200.));

    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(120.));
  }

  #[test]
  fn switch_class_back_to_current_in_same_frame_skip_rebuild() {
    reset_test_env!();

    class_names!(COUNT_CLS);

    let (apply_cnt, w_apply_cnt) = split_value(0usize);
    let (cls, w_cls) = split_value(COUNT_CLS);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      let w_apply_cnt = w_apply_cnt.clone_writer();
      let mut classes = Classes::default();
      classes.insert(COUNT_CLS, |w| {
        *Provider::write_of::<usize>(BuildCtx::get()).unwrap() += 1;
        w
      });

      @Providers {
        providers: smallvec![
          classes.into_provider(),
          Provider::writer(w_apply_cnt.clone_writer(), None),
        ],
        @Container {
          class: pipe!(*$read(cls)),
          hint_size: Size::new(100., 100.),
        }
      }
    });

    wnd.draw_frame();
    assert_eq!(*apply_cnt.read(), 1);

    *w_cls.write() = EMPTY;
    *w_cls.write() = COUNT_CLS;
    wnd.draw_frame();
    assert_eq!(*apply_cnt.read(), 1);
  }

  #[test]
  fn switch_class_not_remount_inner_child() {
    reset_test_env!();

    let (mounted_cnt, w_mounted_cnt) = split_value(0usize);
    let (cls, w_cls) = split_value(EMPTY);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      let mounted_cnt = w_mounted_cnt.clone_writer();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          class: pipe!(*$read(cls)),
          @MockBox {
            size: Size::new(20., 20.),
            on_mounted: move |_| {
              *$write(mounted_cnt) += 1;
            }
          }
        }
      }
    });

    wnd.draw_frame();
    assert_eq!(*mounted_cnt.read(), 1);

    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    assert_eq!(*mounted_cnt.read(), 1);

    *w_cls.write() = EMPTY;
    wnd.draw_frame();
    assert_eq!(*mounted_cnt.read(), 1);
  }

  #[test]

  fn on_disposed_of_class_nodes() {
    reset_test_env!();

    class_names!(ON_DISPOSED);

    let (cls, w_cls) = split_value(ON_DISPOSED);

    use std::sync::atomic::{AtomicBool, Ordering};
    static DISPOSED: AtomicBool = AtomicBool::new(false);
    DISPOSED.store(false, Ordering::Relaxed);

    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      let mut classes = initd_classes();
      classes.insert(ON_DISPOSED, |w| {
        fn_widget! {
          @MockBox {
            size: Size::zero(),
            on_disposed: move |_| DISPOSED.store(true, Ordering::Relaxed),
            @ { w }
          }
        }
        .into_widget()
      });
      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    assert!(!DISPOSED.load(Ordering::Relaxed));

    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    assert!(DISPOSED.load(Ordering::Relaxed));
  }

  #[test]
  fn class_chain() {
    reset_test_env!();

    let wnd = TestWindow::from_widget(fn_widget! {
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: [MARGIN, CLAMP_50],
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(70., 70.));
  }

  #[test]
  fn fix_crash_for_class_impl_may_have_multi_child() {
    reset_test_env!();

    class_names!(MULTI);
    let (cls, w_cls) = split_value(MARGIN);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      let mut classes = initd_classes();
      classes.insert(MULTI, |w| {
        fn_widget! {
          @MockMulti {
            @MockBox { size: Size::new(100., 100.) }
            @MockBox { size: Size::new(100., 200.) }
            @ { w }
          }
        }
        .into_widget()
      });
      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(120., 120.));

    *w_cls.write() = MULTI;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(300., 200.));
  }

  #[test]
  fn fix_provider_in_pipe_class() {
    reset_test_env!();

    class_names!(PROVIDER_CLS);

    let (r_val, w_val) = split_value(-1);
    let wnd = TestWindow::from_widget(fn_widget! {
      let trigger = Stateful::new(true);
      let mut classes = Classes::default();
      classes.insert(PROVIDER_CLS, |w| {
        Providers::new([Provider::new(0i32)])
          .with_child(fn_widget! { w })
      });

      @Providers {
        providers: smallvec![classes.into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!($read(trigger); PROVIDER_CLS),
          on_performed_layout: move |e| {
            *$write(w_val) =  *Provider::of::<i32>(e).unwrap();
          }
        }
      }
    });
    wnd.draw_frame();

    assert_eq!(*r_val.read(), 0);
  }

  #[test]
  fn fix_not_mounted_class_node() {
    reset_test_env!();

    let (cls, w_cls) = split_value(EMPTY);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(100.));

    *w_cls.write() = BOX_200;
    wnd.draw_frame();

    wnd.assert_root_size(Size::splat(200.));
  }

  #[test]
  fn fix_style_class_switch() {
    reset_test_env!();

    let (cls, w_cls) = split_value(EMPTY);
    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(100., 100.));

    *w_cls.write() = CLAMP_50;
    wnd.draw_frame();
    wnd.assert_root_size(Size::new(50., 50.));
  }

  #[test]
  fn override_class() {
    reset_test_env!();

    let wnd = TestWindow::from_widget(fn_widget! {
      @Providers {
        providers: smallvec![
          initd_classes().into_provider(),
          Class::provider(MARGIN, style_class!{
            clamp: BoxClamp::fixed_size(Size::new(66., 66.))
          })
        ],
        @Container {
          hint_size: Size::new(100., 100.),
          class: MARGIN,
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(66., 66.));
  }

  #[test]
  fn fix_pipe_class_on_pipe_widget() {
    reset_test_env!();

    let (w_trigger, w) = split_value(true);
    let (cls, w_cls) = split_value(EMPTY);

    let wnd = TestWindow::from_widget(fn_widget! {
      let w_trigger = w_trigger.clone_watcher();
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![initd_classes().into_provider()],
        @ {
          let w = pipe!(*$read(w_trigger)).map(|_| fn_widget!{
            @Container {hint_size: Size::new(100., 100.) }
          });
          @Class {
            class: pipe!(*$read(cls)),
            @ { w }
          }
        }
      }
    });

    wnd.draw_frame();
    *w.write() = false;
    wnd.draw_frame();
    *w_cls.write() = MARGIN;
    wnd.draw_frame();
    wnd.assert_root_size(Size::splat(120.));
  }

  #[test]
  fn fix_track_id_in_new_class() {
    reset_test_env!();

    class_names!(TRACK_ID);
    let mut classes = initd_classes();
    classes.insert(TRACK_ID, |w| {
      let mut w = FatObj::new(w);
      rdl! {
        @Container {
          hint_size: Size::new(100., 100.),
          @(w) {
            on_performed_layout: move |e| {
              let id = $clone(w.track_id()).get().unwrap();
              assert!(!id.is_dropped(e.tree()));
            }
          }
        }
      }
      .into_widget()
    });

    let (cls, w_cls) = split_value(EMPTY);

    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_watcher();
      @Providers {
        providers: smallvec![classes.clone().into_provider()],
        @Container {
          hint_size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    *w_cls.write() = TRACK_ID;
    wnd.draw_frame();
  }

  #[test]
  fn fix_pipe_class_in_pipe_class() {
    reset_test_env!();

    class_names! { PIPE_CLS, INNER_PIPE_A, INNER_PIPE_B };

    let (cls, w_cls) = split_value(INNER_PIPE_A);
    let (out, w_out) = split_value(EMPTY);
    let wnd = TestWindow::from_widget(fn_widget! {
      let mut classes = Classes::default();
      classes.insert(PIPE_CLS, style_class!{
        class: Variant::<ClassName>::new(BuildCtx::get()).unwrap()
      });

      let out = out.clone_watcher();
      let cls = cls.clone_watcher();
      providers!{
        providers: smallvec![
          classes.clone().into_provider(),
          Provider::watcher(cls.clone_watcher())
        ],
        @MockBox {
          class: pipe!(*$read(out)),
          size: Size::new(100., 100.),
        }
      }
    });
    *w_out.write() = PIPE_CLS;
    wnd.draw_frame();
    *w_cls.write() = INNER_PIPE_B;
    wnd.draw_frame();
  }

  #[test]
  fn fix_pipe_class_unsubscribed() {
    reset_test_env!();

    class_names! { OUT_PIPE_CLS, OUT_PIPE_CLS_2, INNER_PIPE, INNER_PIPE_2};

    let inner_apply = Stateful::new(0usize);
    let w_inner_apply = inner_apply.clone_writer();
    let (inner, w_inner) = split_value(false);
    let (out, w_out) = split_value(OUT_PIPE_CLS);
    let wnd = TestWindow::from_widget(fn_widget! {
      let out_cls = Class::provider(OUT_PIPE_CLS, style_class!{
        class: Variant::<bool>::new(BuildCtx::get()).unwrap()
          .map(|b| if *b { INNER_PIPE } else { INNER_PIPE_2 } )
      });
      let out_cls_2 = Class::provider(OUT_PIPE_CLS_2, style_class!{
        class: Variant::<bool>::new(BuildCtx::get()).unwrap()
          .map(|b| if *b { INNER_PIPE } else { INNER_PIPE_2 } )
      });
      let inner_cls = Class::provider(INNER_PIPE, |w| {
        *Provider::write_of::<usize>(BuildCtx::get()).unwrap() += 1;
        w
      });
      let inner_cls_2 = Class::provider(INNER_PIPE_2, |w| {
        *Provider::write_of::<usize>(BuildCtx::get()).unwrap() += 1;
        w
      });

      let out = out.clone_watcher();
      let inner = inner.clone_watcher();
      let w_inner_apply = w_inner_apply.clone_writer();
      providers!{
        providers: smallvec![
          out_cls, out_cls_2, inner_cls, inner_cls_2,
          Provider::watcher(inner.clone_watcher()),
          Provider::writer(w_inner_apply.clone_writer(), None),
        ],
        @MockBox {
          class: pipe!(*$read(out)),
          size: Size::new(100., 100.),
        }
      }
    });
    wnd.draw_frame();
    assert_eq!(*inner_apply.read(), 1);

    *w_out.write() = OUT_PIPE_CLS_2;
    wnd.draw_frame();
    assert_eq!(*inner_apply.read(), 2);

    *w_inner.write() = true;
    wnd.draw_frame();
    assert_eq!(*inner_apply.read(), 3);
  }

  #[test]
  fn nested_pipe_item_updates_only_target_layer() {
    reset_test_env!();

    class_names!(NESTED_OUTER, NESTED_MID_A, NESTED_MID_B, NESTED_INNER);

    #[derive(Default)]
    struct OuterApply(usize);
    #[derive(Default)]
    struct MidApply(usize);
    #[derive(Default)]
    struct InnerApply(usize);

    let (outer_apply, w_outer_apply) = split_value(OuterApply::default());
    let (mid_apply, w_mid_apply) = split_value(MidApply::default());
    let (inner_apply, w_inner_apply) = split_value(InnerApply::default());
    let (mid_toggle, w_mid_toggle) = split_value(false);

    let wnd = TestWindow::from_widget(fn_widget! {
      let mid_toggle = mid_toggle.clone_watcher();
      let mut classes = Classes::default();
      classes.insert(NESTED_OUTER, |w| {
        Provider::write_of::<OuterApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(NESTED_MID_A, |w| {
        Provider::write_of::<MidApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(NESTED_MID_B, |w| {
        Provider::write_of::<MidApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(NESTED_INNER, |w| {
        Provider::write_of::<InnerApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });

      @Providers {
        providers: smallvec![
          classes.into_provider(),
          Provider::writer(w_outer_apply.clone_writer(), None),
          Provider::writer(w_mid_apply.clone_writer(), None),
          Provider::writer(w_inner_apply.clone_writer(), None),
        ],
        @MockBox {
          size: Size::new(100., 100.),
          class: class_list![
            NESTED_OUTER,
            pipe!(*$read(mid_toggle))
              .map(|v| Some(if v { NESTED_MID_A } else { NESTED_MID_B }))
              .with_init_value(Some(NESTED_MID_B)),
            NESTED_INNER,
          ],
        }
      }
    });
    wnd.draw_frame();
    assert_eq!(outer_apply.read().0, 1);
    assert_eq!(mid_apply.read().0, 1);
    assert_eq!(inner_apply.read().0, 1);

    *w_mid_toggle.write() = true;
    wnd.draw_frame();
    assert_eq!(outer_apply.read().0, 1);
    assert_eq!(mid_apply.read().0, 2);
    assert_eq!(inner_apply.read().0, 1);
  }

  #[test]
  fn nested_pipe_item_equal_short_circuit() {
    reset_test_env!();

    class_names!(NESTED_EQ_OUTER, NESTED_EQ_MID_A, NESTED_EQ_MID_B, NESTED_EQ_INNER);

    #[derive(Default)]
    struct MidApply(usize);

    let (mid_apply, w_mid_apply) = split_value(MidApply::default());
    let (mid_toggle, w_mid_toggle) = split_value(true);

    let wnd = TestWindow::from_widget(fn_widget! {
      let mid_toggle = mid_toggle.clone_watcher();
      let mut classes = Classes::default();
      classes.insert(NESTED_EQ_OUTER, empty_cls);
      classes.insert(NESTED_EQ_INNER, empty_cls);
      classes.insert(NESTED_EQ_MID_A, |w| {
        Provider::write_of::<MidApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(NESTED_EQ_MID_B, |w| {
        Provider::write_of::<MidApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });

      @Providers {
        providers: smallvec![
          classes.into_provider(),
          Provider::writer(w_mid_apply.clone_writer(), None),
        ],
        @MockBox {
          size: Size::new(100., 100.),
          class: class_list![
            NESTED_EQ_OUTER,
            pipe!(*$read(mid_toggle))
              .map(|v| Some(if v { NESTED_EQ_MID_A } else { NESTED_EQ_MID_B }))
              .with_init_value(Some(NESTED_EQ_MID_A)),
            NESTED_EQ_INNER,
          ],
        }
      }
    });
    wnd.draw_frame();
    assert_eq!(mid_apply.read().0, 1);

    *w_mid_toggle.write() = true;
    wnd.draw_frame();
    assert_eq!(mid_apply.read().0, 1);
  }

  #[test]
  fn outer_pipe_equal_rebind_inner_subscriptions() {
    reset_test_env!();

    class_names!(OUTER_EQ_REBIND_OUTER, OUTER_EQ_REBIND_MID_A, OUTER_EQ_REBIND_MID_B);

    #[derive(Default)]
    struct OuterApply(usize);
    #[derive(Default)]
    struct MidApply(usize);

    let (outer_apply, w_outer_apply) = split_value(OuterApply::default());
    let (mid_apply, w_mid_apply) = split_value(MidApply::default());
    let (use_first, w_use_first) = split_value(true);
    let (inner_a, w_inner_a) = split_value(false);
    let (inner_b, w_inner_b) = split_value(false);

    let wnd = TestWindow::from_widget(fn_widget! {
      let use_first = use_first.clone_watcher();
      let inner_a = inner_a.clone_watcher();
      let inner_b = inner_b.clone_watcher();
      let mut classes = Classes::default();
      classes.insert(OUTER_EQ_REBIND_OUTER, |w| {
        Provider::write_of::<OuterApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_EQ_REBIND_MID_A, |w| {
        Provider::write_of::<MidApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_EQ_REBIND_MID_B, |w| {
        Provider::write_of::<MidApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });

      @Providers {
        providers: smallvec![
          classes.into_provider(),
          Provider::writer(w_outer_apply.clone_writer(), None),
          Provider::writer(w_mid_apply.clone_writer(), None),
        ],
        @MockBox {
          size: Size::new(100., 100.),
          class: pipe!(*$read(use_first)).map(move |use_first| {
            let source = if use_first {
              pipe!(*$read(inner_a))
                .map(|v| Some(if v { OUTER_EQ_REBIND_MID_A } else { OUTER_EQ_REBIND_MID_B }))
                .with_init_value(Some(OUTER_EQ_REBIND_MID_B))
            } else {
              pipe!(*$read(inner_b))
                .map(|v| Some(if v { OUTER_EQ_REBIND_MID_A } else { OUTER_EQ_REBIND_MID_B }))
                .with_init_value(Some(OUTER_EQ_REBIND_MID_B))
            };
            class_list![OUTER_EQ_REBIND_OUTER, source]
          }),
        }
      }
    });
    wnd.draw_frame();
    let outer_after_first = outer_apply.read().0;
    let mid_after_first = mid_apply.read().0;
    assert_eq!(outer_after_first, 1);
    assert_eq!(mid_after_first, 1);

    *w_use_first.write() = false;
    wnd.draw_frame();
    assert_eq!(outer_apply.read().0, outer_after_first);
    assert_eq!(mid_apply.read().0, mid_after_first);

    *w_inner_b.write() = true;
    wnd.draw_frame();
    assert_eq!(mid_apply.read().0, mid_after_first + 1);

    *w_inner_a.write() = true;
    wnd.draw_frame();
    assert_eq!(mid_apply.read().0, mid_after_first + 1);
  }

  #[test]
  fn outer_pipe_shape_change_rebuild() {
    reset_test_env!();

    class_names!(OUTER_SHAPE_A, OUTER_SHAPE_B, OUTER_SHAPE_C);

    #[derive(Default)]
    struct AApply(usize);
    #[derive(Default)]
    struct BApply(usize);
    #[derive(Default)]
    struct CApply(usize);

    let (a_apply, w_a_apply) = split_value(AApply::default());
    let (b_apply, w_b_apply) = split_value(BApply::default());
    let (c_apply, w_c_apply) = split_value(CApply::default());
    let (with_tail, w_with_tail) = split_value(true);

    let wnd = TestWindow::from_widget(fn_widget! {
      let with_tail = with_tail.clone_watcher();
      let mut classes = Classes::default();
      classes.insert(OUTER_SHAPE_A, |w| {
        Provider::write_of::<AApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_SHAPE_B, |w| {
        Provider::write_of::<BApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_SHAPE_C, |w| {
        Provider::write_of::<CApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });

      @Providers {
        providers: smallvec![
          classes.into_provider(),
          Provider::writer(w_a_apply.clone_writer(), None),
          Provider::writer(w_b_apply.clone_writer(), None),
          Provider::writer(w_c_apply.clone_writer(), None),
        ],
        @MockBox {
          size: Size::new(100., 100.),
          class: pipe!(*$read(with_tail)).map(|with_tail| {
            if with_tail {
              class_list![OUTER_SHAPE_A, OUTER_SHAPE_B, OUTER_SHAPE_C]
            } else {
              class_list![OUTER_SHAPE_A, OUTER_SHAPE_B]
            }
          }),
        }
      }
    });
    wnd.draw_frame();
    let a_first = a_apply.read().0;
    let b_first = b_apply.read().0;
    let c_first = c_apply.read().0;
    assert_eq!(a_first, 1);
    assert_eq!(b_first, 1);
    assert_eq!(c_first, 1);

    *w_with_tail.write() = false;
    wnd.draw_frame();
    assert_eq!(a_apply.read().0, a_first + 1);
    assert_eq!(b_apply.read().0, b_first + 1);
    assert_eq!(c_apply.read().0, c_first);
  }

  #[test]
  fn outer_and_inner_pipe_together() {
    reset_test_env!();

    class_names!(OUTER_INNER_A, OUTER_INNER_B_A, OUTER_INNER_B_B, OUTER_INNER_C);

    #[derive(Default)]
    struct AApply(usize);
    #[derive(Default)]
    struct BApply(usize);
    #[derive(Default)]
    struct CApply(usize);

    let (a_apply, w_a_apply) = split_value(AApply::default());
    let (b_apply, w_b_apply) = split_value(BApply::default());
    let (c_apply, w_c_apply) = split_value(CApply::default());
    let (with_tail, w_with_tail) = split_value(true);
    let (mid_toggle, w_mid_toggle) = split_value(false);

    let wnd = TestWindow::from_widget(fn_widget! {
      let with_tail = with_tail.clone_watcher();
      let mid_toggle = mid_toggle.clone_watcher();
      let mut classes = Classes::default();
      classes.insert(OUTER_INNER_A, |w| {
        Provider::write_of::<AApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_INNER_B_A, |w| {
        Provider::write_of::<BApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_INNER_B_B, |w| {
        Provider::write_of::<BApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });
      classes.insert(OUTER_INNER_C, |w| {
        Provider::write_of::<CApply>(BuildCtx::get()).unwrap().0 += 1;
        w
      });

      @Providers {
        providers: smallvec![
          classes.into_provider(),
          Provider::writer(w_a_apply.clone_writer(), None),
          Provider::writer(w_b_apply.clone_writer(), None),
          Provider::writer(w_c_apply.clone_writer(), None),
        ],
        @MockBox {
          size: Size::new(100., 100.),
          class: pipe!(*$read(with_tail)).map(move |with_tail| {
            let mid = pipe!(*$read(mid_toggle))
              .map(|v| Some(if v { OUTER_INNER_B_A } else { OUTER_INNER_B_B }))
              .with_init_value(Some(OUTER_INNER_B_B));
            if with_tail {
              class_list![OUTER_INNER_A, mid, OUTER_INNER_C]
            } else {
              class_list![OUTER_INNER_A, mid]
            }
          }),
        }
      }
    });
    wnd.draw_frame();
    let a_start = a_apply.read().0;
    let b_start = b_apply.read().0;
    let c_start = c_apply.read().0;
    assert_eq!(a_start, 1);
    assert_eq!(b_start, 1);
    assert_eq!(c_start, 1);

    *w_mid_toggle.write() = true;
    wnd.draw_frame();

    *w_with_tail.write() = false;
    wnd.draw_frame();

    *w_mid_toggle.write() = false;
    wnd.draw_frame();

    *w_with_tail.write() = true;
    wnd.draw_frame();
    assert_eq!(a_apply.read().0, a_start + 2);
    assert_eq!(c_apply.read().0, c_start + 1);
    assert!(b_apply.read().0 >= b_start + 2);

    let b_before_final_inner_update = b_apply.read().0;
    *w_mid_toggle.write() = true;
    wnd.draw_frame();
    assert_eq!(a_apply.read().0, a_start + 2);
    assert_eq!(c_apply.read().0, c_start + 1);
    assert!(b_apply.read().0 > b_before_final_inner_update);
  }

  // the track_id is bind after the class, when the class is changed and wrap with
  // new reader(here is the margin), the track_id should changed.
  #[test]
  fn fix_track_id_in_class_node() {
    reset_test_env!();

    class_names! { WRAP_CLS, IDENTITY_CLS };

    let (r_cls, w_cls) = split_value(IDENTITY_CLS);
    let (r_id, w_id) = split_value(None);
    let w = fn_widget! {
      let cls = Class::provider(WRAP_CLS, style_class!(
        margin: EdgeInsets::all(2.),
      ));

      let mut w = FatObj::new(
        @Void {
          class: pipe!(*$read(r_cls)),
        }.into_widget()
      );
      *$write(w_id) = Some($clone(w.track_id()));

      @Providers{
        providers: smallvec![
          cls,
        ],
        @ { w }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    let id1 = r_id.read().as_ref().and_then(|w| w.get());

    *w_cls.write() = WRAP_CLS;
    wnd.draw_frame();

    let id2 = r_id.read().as_ref().and_then(|w| w.get());
    assert!(id1 != id2);
    assert!(!id2.unwrap().is_dropped(wnd.tree()));
  }

  #[test]
  fn override_size_by_widget_field() {
    reset_test_env!();

    let mut classes = Classes::default();
    classes.insert(
      CLAMP_50,
      style_class! {
        clamp: BoxClamp::fixed_size(Size::new(50., 50.))
      },
    );

    let cls = Stateful::new(Some(CLAMP_50));

    let wnd = TestWindow::from_widget(fn_widget! {
      let cls = cls.clone_writer();
      @Providers {
        providers: smallvec![classes.clone().into_provider()],
        @Container {
          size: Size::new(100., 100.),
          class: pipe!(*$read(cls)),
        }
      }
    });

    wnd.draw_frame();
    wnd.assert_root_size(Size::new(100., 100.));
  }

  #[test]
  fn fix_pipe_parent_with_pipe_class() {
    reset_test_env!();

    class_names!(CLS_A, CLS_B);

    let (expanded, w_expanded) = split_value(true);
    let (cls_toggle, w_cls) = split_value(true);

    let w = fn_widget! {
      let mut classes = Classes::default();
      // CLS_A and CLS_B use margin to create real wrapper nodes,
      // matching gallery behavior (RAIL_ITEM_SELECTED/UNSELECTED use style_class with real props).
      classes.insert(CLS_A, style_class! { margin: EdgeInsets::all(1.) });
      classes.insert(CLS_B, style_class! { margin: EdgeInsets::all(2.) });

      // Pipe parent: switches between MockMulti (horizontal) and MockStack (overlap)
      let pipe_parent = pipe!(*$read(expanded)).map(move |is_horiz| {
        if is_horiz {
          MockMulti.into_multi_child()
        } else {
          MockStack {}.into_multi_child()
        }
      }).into_multi_child();

      // Pipe class
      let cls = pipe!(*$read(cls_toggle)).map(|v| {
        if v { CLS_A } else { CLS_B }
      });

      let mut obj = FatObj::new(pipe_parent);
      obj.with_class(cls);

      @Providers {
        providers: smallvec::smallvec![Provider::new(classes)],
        @(obj) {
          @MockBox { size: Size::new(10., 10.) }
          @MockBox { size: Size::new(10., 10.) }
        }
      }
    };

    let wnd = TestWindow::from_widget(w);
    wnd.draw_frame();
    // MockMulti + margin 1 on each side: width=20+2, height=10+2
    wnd.assert_root_size(Size::new(22., 12.));

    // Toggle pipe parent: MockMulti -> MockStack
    *w_expanded.write() = false;
    wnd.draw_frame();
    // MockStack + margin 1: width=10+2, height=10+2
    wnd.assert_root_size(Size::new(12., 12.));

    // Toggle class: CLS_A -> CLS_B (margin 1 -> margin 2)
    *w_cls.write() = false;
    wnd.draw_frame();
    // MockStack + margin 2: width=10+4, height=10+4
    wnd.assert_root_size(Size::new(14., 14.));

    // Toggle pipe parent back: MockStack -> MockMulti
    *w_expanded.write() = true;
    wnd.draw_frame();
    // MockMulti + margin 2: width=20+4, height=10+4
    wnd.assert_root_size(Size::new(24., 14.));

    // Simultaneous: toggle both class and parent on the same frame
    *w_cls.write() = true; // CLS_B -> CLS_A (margin 2 -> margin 1)
    *w_expanded.write() = false; // MockMulti -> MockStack
    wnd.draw_frame();
    // MockStack + margin 1: width=10+2, height=10+2
    wnd.assert_root_size(Size::new(12., 12.));
  }
}
