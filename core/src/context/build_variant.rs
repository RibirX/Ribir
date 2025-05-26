use crate::prelude::*;
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

/// `VariantMap` is a Variant that maps a value to another value using a
/// function.
#[derive(Clone)]
pub struct VariantMap<V: 'static, F> {
  variant: Variant<V>,
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
  pub fn map<F, U>(self, map: F) -> VariantMap<V, F>
  where
    F: Fn(&V) -> U,
  {
    VariantMap { variant: self, map }
  }

  /// Clones the value of the variant.
  pub fn clone_value(&self) -> V
  where
    V: Clone,
  {
    match self {
      Variant::Value(v) => v.clone(),
      Variant::Watcher(v) => v.read().clone(),
    }
  }

  pub fn map_with_watcher<W, U: 'static>(
    self, w: impl StateWatcher<Value = W>, f: impl Fn(&V, &W) -> U + 'static,
  ) -> Pipe<U> {
    match self {
      Variant::Watcher(v) => pipe!(f(&$v, &$w)),
      Variant::Value(v) => pipe!(f(&v, &$w)),
    }
  }
}

impl Variant<Color> {
  /// Convert a color variant to another color variant with its base lightness
  /// tone.
  pub fn into_base_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(&Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().base;
    self.map(move |c| c.with_lightness(lightness))
  }

  /// Converts a color variant to another color variant with its container
  /// lightness tone.
  pub fn into_container_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(&Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().container;
    self.map(move |c| c.with_lightness(lightness))
  }

  /// Converts a color variant to another color variant that its lightness tone
  /// is suitable display on its base color.
  pub fn on_this_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(&Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().on;
    self.map(move |c| c.with_lightness(lightness))
  }

  /// Converts a color variant to another color variant that its lightness tone
  /// is suitable display on its container color.
  pub fn on_this_container_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(&Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().on_container;
    self.map(move |c| c.with_lightness(lightness))
  }
}

impl<V, F> VariantMap<V, F> {
  /// Maps a value to another value using a function.
  pub fn map<F2, U1, U2>(self, map: F2) -> VariantMap<V, impl Fn(&V) -> U2>
  where
    F: Fn(&V) -> U1,
    F2: Fn(U1) -> U2,
  {
    VariantMap { variant: self.variant, map: move |v: &V| map((self.map)(v)) }
  }

  /// Clones the value of the variant.
  pub fn clone_value<U>(&self) -> U
  where
    F: Fn(&V) -> U,
    V: Clone,
  {
    (self.map)(&self.variant.clone_value())
  }
}

pub struct VariantKind<K: ?Sized>(PhantomData<fn() -> K>);
impl<V: Clone + 'static, U, K: ?Sized + 'static> RFrom<Variant<V>, VariantKind<K>> for PipeValue<U>
where
  U: RFrom<V, K> + 'static,
{
  fn r_from(value: Variant<V>) -> Self {
    match value {
      Variant::Watcher(value) => {
        let init = value.read().clone().r_into();
        pipe!(U::r_from($value.clone())).with_init_value(init)
      }
      Variant::Value(value) => value.r_into(),
    }
  }
}

impl<V, F, U, P, K: ?Sized> RFrom<VariantMap<V, F>, VariantKind<K>> for PipeValue<P>
where
  F: Fn(&V) -> U + 'static,
  P: RFrom<U, K> + 'static,
{
  fn r_from(value: VariantMap<V, F>) -> Self {
    match value.variant {
      Variant::Watcher(s) => {
        let init = (value.map)(&s.read()).r_into();
        pipe!(P::r_from((value.map)(&$s)))
          .with_init_value(init)
          .r_into()
      }
      Variant::Value(v) => (value.map)(&v).r_into(),
    }
  }
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
      Variant::Watcher(w) => pipe!($w.clone().r_into()).into_widget(),
      Variant::Value(v) => v.r_into(),
    }
  }
}

impl<V, F, U, K: ?Sized> RFrom<VariantMap<V, F>, OtherWidget<K>> for Widget<'static>
where
  F: Fn(&V) -> U + 'static,
  U: RInto<Widget<'static>, K> + 'static,
{
  fn r_from(value: VariantMap<V, F>) -> Self {
    let VariantMap { variant, map } = value;
    match variant {
      Variant::Watcher(w) => pipe!(map(&$w)).build_single(),
      Variant::Value(v) => map(&v).r_into(),
    }
  }
}
