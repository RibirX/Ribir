//! Variant composition utilities for build-time providers.
//!
//! `Variant` represents a value that can be either constant or watcher-backed.
//! `VariantMap` and `Combine` provide a lightweight composition model:
//! - `map` transforms a single source.
//! - `combine` pairs two sources.
//! - `combine_with` pairs and maps in one step.
//!
//! All sources can be converted into `PipeValue` with a correct init value.
//! Use `snapshot` for a point-in-time read, and `freeze` to drop reactivity.

use std::convert::Infallible;

use rxrust::observable::boxed::LocalBoxedObservableClone;

use crate::prelude::*;

/// Convert a watcher-like object into a `Variant`.
///
/// This enables APIs to accept `StateWatcher`/`Stateful` without forcing
/// callers to explicitly wrap them.
pub trait IntoVariant {
  type Value: 'static;

  fn into_variant(self) -> Variant<Self::Value>;
}

impl<V: 'static> IntoVariant for Variant<V> {
  type Value = V;

  fn into_variant(self) -> Variant<<Self as IntoVariant>::Value> { self }
}

impl<T> IntoVariant for T
where
  T: StateWatcher + 'static,
  T::Value: Sized + 'static,
{
  type Value = T::Value;

  fn into_variant(self) -> Variant<Self::Value> { Variant::from_watcher(self) }
}

/// A source that can participate in `Variant` composition.
///
/// This is the internal abstraction used by `Variant`, `VariantMap`, and
/// combination nodes. Snapshot support is provided by [`VariantSnapshot`].
pub trait VariantSource: Sized + 'static {
  type Value: 'static;

  /// Observable modifies stream that drives updates, if any.
  #[doc(hidden)]
  fn modifies(&self) -> Option<LocalBoxedObservableClone<'static, ModifyInfo, Infallible>>;

  /// Map the source value to another value.
  fn map<F, U>(self, map: F) -> VariantMap<Self, F>
  where
    F: Fn(&Self::Value) -> U,
  {
    VariantMap { source: self, map }
  }

  /// Combine with another source.
  fn combine<R>(self, rhs: R) -> Combine<Self, R::Source>
  where
    R: VariantInput,
  {
    Combine { left: self, right: rhs.into_source() }
  }

  /// Combine with another source, then map into a new value.
  fn combine_with<R, F, U: 'static>(self, rhs: R, f: F) -> CombineWithMap<Self, R::Source, F>
  where
    R: VariantInput,
    F: Fn(&(Self::Value, R::Value)) -> U + 'static,
  {
    self.combine(rhs).map(f)
  }
}

/// A `VariantSource` that supports snapshot and pipe conversion.
pub trait VariantSnapshot: VariantSource
where
  <Self as VariantSource>::Value: Clone,
{
  /// Snapshot current value of the source.
  fn snapshot(&self) -> <Self as VariantSource>::Value;

  /// Convert this source into a `PipeValue`, preserving init value.
  fn into_pipe_value(self) -> PipeValue<<Self as VariantSource>::Value>
  where
    Self: Sized,
  {
    let init_value = self.snapshot();
    match self.modifies() {
      Some(modifies) => {
        let trigger = modifies.box_it();
        let source = self;
        let pipe = Pipe::new(trigger, move |_| source.snapshot());
        PipeValue::Pipe { init_value, pipe }
      }
      None => PipeValue::Value(init_value),
    }
  }

  /// Freeze the source into a constant `Variant`.
  fn freeze(self) -> Variant<<Self as VariantSource>::Value>
  where
    Self: Sized,
  {
    Variant::Value(self.snapshot())
  }
}

/// Type alias for the mapped result of `VariantSource::combine_with`.
pub type CombineWithMap<L, R, F> = VariantMap<Combine<L, R>, F>;

/// Input type for `combine` and `combine_with`.
pub trait VariantInput {
  type Value: 'static;
  type Source: VariantSource<Value = Self::Value>;

  fn into_source(self) -> Self::Source;
}

impl<T> VariantInput for T
where
  T: IntoVariant,
{
  type Value = T::Value;
  type Source = Variant<Self::Value>;

  fn into_source(self) -> Self::Source { self.into_variant() }
}

impl<S, F, U> VariantInput for VariantMap<S, F>
where
  S: VariantSource,
  U: 'static,
  F: Fn(&S::Value) -> U + 'static,
{
  type Value = U;
  type Source = VariantMap<S, F>;

  fn into_source(self) -> Self::Source { self }
}

impl<L, R> VariantInput for Combine<L, R>
where
  L: VariantSource,
  R: VariantSource,
{
  type Value = (L::Value, R::Value);
  type Source = Combine<L, R>;

  fn into_source(self) -> Self::Source { self }
}

/// Combined source of two `Variant`-like values.
pub struct Combine<L, R> {
  left: L,
  right: R,
}

impl<L, R> Combine<L, R>
where
  L: VariantSource,
  R: VariantSource,
{
  /// Maps a combined value to another value using a function.
  pub fn map<F, U>(self, map: F) -> VariantMap<Combine<L, R>, F>
  where
    F: Fn(&(L::Value, R::Value)) -> U,
  {
    VariantMap { source: self, map }
  }
}
/// `Variant` is an enum designed to help you store a clone of a provider. It
/// serves as a shortcut for `Provider::state_of` and `Provider::of`.
///
/// Initially, it checks for the existence of a watcher provider; if not
/// found, it proceeds to check the value provider.
///
/// It supports conversion to `PipeValue` for initialization of a declare
/// object, enabling the object to track changes in the provider value if it's a
/// watcher provider.
///
/// ## Example
///
/// ```
/// use ribir_core::prelude::*;
///
/// let _ = fn_widget! {
///   let color = Variant::<Color>::new(BuildCtx::get()).unwrap();
///   @Container {
///     size: Size::new(100., 100.),
///     background: color,
///   }
/// };
/// ```
///
/// Here, we create a 100x100 rectangle with a background using the `Color`
/// Provider. If an ancestor provides a writer of `Color`, this rectangle will
/// reflect changes in color.
pub enum Variant<V> {
  Watcher(Box<dyn StateWatcher<Value = V>>),
  Value(V),
}

/// `VariantMap` is a mapped `VariantSource`.
///
/// It preserves reactivity from its source while transforming values through
/// a mapping function.
#[derive(Clone)]
pub struct VariantMap<S, F> {
  source: S,
  map: F,
}

impl<V: 'static> Variant<V> {
  /// Creates a new `Variant` from a provider context.
  pub fn new(ctx: &impl AsRef<ProviderCtx>) -> Option<Self>
  where
    V: Clone,
  {
    if let Some(value) = Provider::state_of::<Box<dyn StateWatcher<Value = V>>>(ctx) {
      Some(Variant::Watcher(value.clone_boxed_watcher()))
    } else {
      Provider::of::<V>(ctx).map(|v| Variant::Value(v.clone()))
    }
  }

  /// Creates a new `Variant` from a provider context or uses the default value
  /// if it's not found.
  pub fn new_or(ctx: &impl AsRef<ProviderCtx>, default: V) -> Self
  where
    V: Clone,
  {
    Self::new(ctx).unwrap_or(Variant::Value(default))
  }

  /// Creates a new `Variant` from a provider context or uses the `f` closure
  /// to create a default value if the context lookup fails.
  pub fn new_or_else(ctx: &impl AsRef<ProviderCtx>, f: impl FnOnce() -> V) -> Self
  where
    V: Clone,
  {
    Self::new(ctx).unwrap_or_else(|| Variant::Value(f()))
  }

  /// Creates a new `Variant` from a provider context or uses the default value
  /// if it's not found.
  pub fn new_or_default(ctx: &impl AsRef<ProviderCtx>) -> Self
  where
    V: Default + Clone,
  {
    Self::new_or_else(ctx, V::default)
  }

  /// Creates a new `Variant` from a watcher.
  pub fn from_watcher(watcher: impl StateWatcher<Value = V>) -> Self {
    match watcher.try_into_value() {
      Ok(v) => Variant::Value(v),
      Err(w) => Variant::Watcher(w.clone_boxed_watcher()),
    }
  }

  /// Maps a value to another value using a function.
  pub fn map<F, U>(self, map: F) -> VariantMap<Variant<V>, F>
  where
    F: Fn(&V) -> U,
  {
    VariantMap { source: self, map }
  }

  /// Snapshot current value of the variant.
  pub fn snapshot(&self) -> V
  where
    V: Clone,
  {
    match self {
      Variant::Value(v) => v.clone(),
      Variant::Watcher(v) => v.read().clone(),
    }
  }
}

impl<V> VariantSource for Variant<V>
where
  V: 'static,
{
  type Value = V;

  fn modifies(&self) -> Option<LocalBoxedObservableClone<'static, ModifyInfo, Infallible>> {
    match self {
      Variant::Watcher(watcher) => Some(watcher.modifies()),
      Variant::Value(_) => None,
    }
  }
}

impl<V> VariantSnapshot for Variant<V>
where
  V: Clone + 'static,
{
  fn snapshot(&self) -> V {
    match self {
      Variant::Value(v) => v.clone(),
      Variant::Watcher(v) => v.read().clone(),
    }
  }
}

type ColorMap<S> =
  VariantMap<Combine<S, Variant<LightnessTone>>, fn(&(Color, LightnessTone)) -> Color>;

fn apply_lightness(input: &(Color, LightnessTone)) -> Color { input.0.with_lightness(input.1) }

/// Color-specific helpers for any `VariantSource<Color>`.
pub trait VariantColorExt: VariantSource<Value = Color> + Sized {
  /// Convert a color variant to another color variant with its base lightness
  /// tone.
  fn into_base_color(self, ctx: &impl AsRef<ProviderCtx>) -> ColorMap<Self> {
    let palette = Palette::of(ctx);
    let lightness = palette.lightness_group().base;
    self
      .combine(Variant::Value(lightness))
      .map(apply_lightness)
  }

  /// Converts a color variant to another color variant with its container
  /// lightness tone.
  fn into_container_color(self, ctx: &impl AsRef<ProviderCtx>) -> ColorMap<Self> {
    let palette = Palette::of(ctx);
    let lightness = palette.lightness_group().container;
    self
      .combine(Variant::Value(lightness))
      .map(apply_lightness)
  }

  /// Converts a color variant to another color variant that its lightness tone
  /// is suitable display on its base color.
  fn on_this_color(self, ctx: &impl AsRef<ProviderCtx>) -> ColorMap<Self> {
    let palette = Palette::of(ctx);
    let lightness = palette.lightness_group().on;
    self
      .combine(Variant::Value(lightness))
      .map(apply_lightness)
  }

  /// Converts a color variant to another color variant that its lightness tone
  /// is suitable display on its container color.
  fn on_this_container_color(self, ctx: &impl AsRef<ProviderCtx>) -> ColorMap<Self> {
    let palette = Palette::of(ctx);
    let lightness = palette.lightness_group().on_container;
    self
      .combine(Variant::Value(lightness))
      .map(apply_lightness)
  }
}

impl<T> VariantColorExt for T where T: VariantSource<Value = Color> + Sized {}

impl<S, F, U> VariantMap<S, F>
where
  S: VariantSource,
  F: Fn(&S::Value) -> U,
{
  /// Maps a value to another value using a function.
  pub fn map<F2, U2>(self, map: F2) -> VariantMap<S, impl Fn(&S::Value) -> U2>
  where
    F2: Fn(U) -> U2,
  {
    let VariantMap { source, map: previous_map } = self;
    VariantMap { source, map: move |value: &S::Value| map(previous_map(value)) }
  }

  /// Snapshot current value of the variant.
  pub fn snapshot(&self) -> U
  where
    S: VariantSnapshot,
    <S as VariantSource>::Value: Clone,
    U: Clone,
  {
    (self.map)(&self.source.snapshot())
  }
}

impl<S, F> crate::widget_children::sealed::IntoXChild<'static, SingleKind> for VariantMap<S, F>
where
  S: VariantSnapshot,
  <S as VariantSource>::Value: Clone,
  F: Fn(&S::Value) -> XSingleChild<'static> + 'static,
{
  fn into_x_child(self) -> XSingleChild<'static> {
    let VariantMap { source, map } = self;
    if let Some(modifies) = source.modifies() {
      let trigger = modifies
        .merge(Local::of(ModifyInfo::default()))
        .box_it();
      Pipe::new(trigger, move |_| map(&source.snapshot())).into_single_child()
    } else {
      map(&source.snapshot())
    }
  }
}

impl<S, F> crate::widget_children::sealed::IntoXChild<'static, MultiKind> for VariantMap<S, F>
where
  S: VariantSnapshot,
  <S as VariantSource>::Value: Clone,
  F: Fn(&S::Value) -> XMultiChild<'static> + 'static,
{
  fn into_x_child(self) -> XMultiChild<'static> {
    let VariantMap { source, map } = self;
    if let Some(modifies) = source.modifies() {
      // Keep children stable by using the pipe-parent replacement path, and emit
      // once immediately so the correct parent is built for the first frame.
      let trigger = modifies
        .merge(Local::of(ModifyInfo::default()))
        .box_it();
      Pipe::new(trigger, move |_| map(&source.snapshot())).into_multi_child()
    } else {
      map(&source.snapshot())
    }
  }
}

impl<S, F, U> VariantSource for VariantMap<S, F>
where
  S: VariantSource,
  U: 'static,
  F: Fn(&S::Value) -> U + 'static,
{
  type Value = U;

  fn modifies(&self) -> Option<LocalBoxedObservableClone<'static, ModifyInfo, Infallible>> {
    self.source.modifies()
  }
}

impl<S, F, U> VariantSnapshot for VariantMap<S, F>
where
  S: VariantSnapshot,
  <S as VariantSource>::Value: Clone,
  U: Clone + 'static,
  F: Fn(&S::Value) -> U + 'static,
{
  fn snapshot(&self) -> Self::Value { (self.map)(&self.source.snapshot()) }
}

impl<L, R> VariantSource for Combine<L, R>
where
  L: VariantSource,
  R: VariantSource,
{
  type Value = (L::Value, R::Value);

  fn modifies(&self) -> Option<LocalBoxedObservableClone<'static, ModifyInfo, Infallible>> {
    match (self.left.modifies(), self.right.modifies()) {
      (Some(left), Some(right)) => Some(left.merge(right).box_it_clone()),
      (Some(left), None) => Some(left),
      (None, Some(right)) => Some(right),
      (None, None) => None,
    }
  }
}

impl<L, R> VariantSnapshot for Combine<L, R>
where
  L: VariantSnapshot,
  <L as VariantSource>::Value: Clone,
  R: VariantSnapshot,
  <R as VariantSource>::Value: Clone,
{
  fn snapshot(&self) -> Self::Value { (self.left.snapshot(), self.right.snapshot()) }
}

pub struct VariantKind<K: ?Sized>(PhantomData<fn() -> K>);
impl<S, U, K: ?Sized + 'static> RFrom<S, VariantKind<K>> for PipeValue<U>
where
  S: VariantSnapshot,
  <S as VariantSource>::Value: Clone,
  U: RFrom<S::Value, K> + 'static,
{
  fn r_from(value: S) -> Self { value.into_pipe_value().map(U::r_from) }
}

impl<V: Clone + 'static> Clone for Variant<V> {
  fn clone(&self) -> Self {
    match self {
      Variant::Watcher(value) => Variant::Watcher(value.clone_boxed_watcher()),
      Variant::Value(value) => Variant::Value(value.clone()),
    }
  }
}

impl<V, K: ?Sized> RFrom<Variant<V>, OtherWidget<K>> for Widget<'static>
where
  V: RInto<Widget<'static>, K> + Clone + 'static,
{
  fn r_from(value: Variant<V>) -> Self {
    match value {
      Variant::Watcher(w) => pipe!($read(w).clone().r_into()).into_widget(),
      Variant::Value(v) => v.r_into(),
    }
  }
}

impl<S, F, U, K: ?Sized> RFrom<VariantMap<S, F>, OtherWidget<K>> for Widget<'static>
where
  S: VariantSnapshot,
  <S as VariantSource>::Value: Clone,
  F: Fn(&S::Value) -> U + 'static,
  U: RInto<Widget<'static>, K> + 'static,
{
  fn r_from(value: VariantMap<S, F>) -> Self {
    let VariantMap { source, map } = value;
    if let Some(modifies) = source.modifies() {
      // Emit once to build the initial widget, then update on modifications.
      let trigger = modifies
        .merge(Local::of(ModifyInfo::default()))
        .box_it();
      let pipe = Pipe::new(trigger, move |_| map(&source.snapshot()));
      pipe.build_single()
    } else {
      map(&source.snapshot()).r_into()
    }
  }
}
