use crate::prelude::*;
/// `Variant` is an enum designed to help you store a clone of a provider. It
/// serves as a shortcut for `Provider::state_of` and `Provider::of`.
///
/// Initially, it checks for the existence of a stateful provider; if not
/// found, it proceeds to check the value provider.
///
/// It supports conversion to `DeclareInit` for initialization of a declare
/// object, enabling the object to track changes in the provider value if it's a
/// stateful provider.
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
/// Provider. If an ancestor provides a `Stateful<Color>`, this rectangle will
/// reflect changes in color.
pub enum Variant<V> {
  Stateful(Stateful<V>),
  Value(V),
}

/// `VariantMap` is a Variant that maps a value to another value using a
/// function.
#[derive(Clone)]
pub struct VariantMap<V: 'static, F> {
  variant: Variant<V>,
  map: F,
}

impl<V: Clone + 'static> Variant<V> {
  /// Creates a new `Variant` from a provider context.
  pub fn new(ctx: &impl AsRef<ProviderCtx>) -> Option<Self> {
    if let Some(value) = Provider::state_of::<Stateful<V>>(ctx) {
      Some(Variant::Stateful(value.clone_writer()))
    } else {
      Provider::of::<V>(ctx).map(|v| Variant::Value(v.clone()))
    }
  }

  /// Maps a value to another value using a function.
  pub fn map<F, U>(self, map: F) -> VariantMap<V, F>
  where
    F: Fn(V) -> U,
  {
    VariantMap { variant: self, map }
  }

  /// Clones the value of the variant.
  pub fn clone_value(&self) -> V {
    match self {
      Variant::Value(v) => v.clone(),
      Variant::Stateful(v) => v.read().clone(),
    }
  }
}

impl Variant<Color> {
  /// Convert a color variant to another color variant with its base lightness
  /// tone.
  pub fn into_base_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().base;
    self.map(move |c| c.with_lightness(lightness))
  }

  /// Converts a color variant to another color variant with its container
  /// lightness tone.
  pub fn into_container_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().container;
    self.map(move |c| c.with_lightness(lightness))
  }

  /// Converts a color variant to another color variant that its lightness tone
  /// is suitable display on its base color.
  pub fn on_this_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().on;
    self.map(move |c| c.with_lightness(lightness))
  }

  /// Converts a color variant to another color variant that its lightness tone
  /// is suitable display on its container color.
  pub fn on_this_container_color(
    self, ctx: &impl AsRef<ProviderCtx>,
  ) -> VariantMap<Color, impl Fn(Color) -> Color> {
    let p = Palette::of(ctx);
    let lightness = p.lightness_group().on_container;
    self.map(move |c| c.with_lightness(lightness))
  }
}

impl<V, F> VariantMap<V, F> {
  /// Maps a value to another value using a function.
  pub fn map<F2, U1, U2>(self, map: F2) -> VariantMap<V, impl Fn(V) -> U2>
  where
    F: Fn(V) -> U1,
    F2: Fn(U1) -> U2,
  {
    VariantMap { variant: self.variant, map: move |v| map((self.map)(v)) }
  }

  /// Clones the value of the variant.
  pub fn clone_value<U>(&self) -> U
  where
    F: Fn(V) -> U,
    V: Clone,
  {
    (self.map)(self.variant.clone_value())
  }
}

impl<V: Clone + 'static, U> DeclareFrom<Variant<V>, 0> for DeclareInit<U>
where
  U: From<V> + 'static,
{
  fn declare_from(value: Variant<V>) -> Self {
    match value {
      Variant::Stateful(value) => pipe!($value.clone()).declare_into(),
      Variant::Value(value) => DeclareInit::Value(value.into()),
    }
  }
}

impl<V: Clone + 'static, F, U, P> DeclareFrom<VariantMap<V, F>, 0> for DeclareInit<P>
where
  F: Fn(V) -> U + 'static,
  P: From<U> + 'static,
{
  fn declare_from(value: VariantMap<V, F>) -> Self {
    match value.variant {
      Variant::Stateful(s) => pipe!(P::from((value.map)($s.clone()))).declare_into(),
      Variant::Value(v) => DeclareInit::Value((value.map)(v).into()),
    }
  }
}

impl<V: Clone + 'static> Clone for Variant<V> {
  fn clone(&self) -> Self {
    match self {
      Variant::Stateful(value) => Variant::Stateful(value.clone_writer()),
      Variant::Value(value) => Variant::Value(value.clone()),
    }
  }
}
