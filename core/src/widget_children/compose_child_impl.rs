use super::*;

/// The trait is used to enable child composition for `ComposeChild`.
pub trait ComposeWithChild<C, K: ?Sized> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

// ------ With child implementations ------
/// ComposeChild compose a type that can convert to its specific child type.
///
/// We choose to return a pair of parent and child instead of directly composing
/// and returning a `Widget`. This approach allows for continued composition
/// with certain child types like `Vec`.
pub struct ChildKind<K: ?Sized>(PhantomData<fn() -> K>);

pub struct TmlKind<K: ?Sized>(PhantomData<fn() -> K>);

impl<'c, P, C, K: ?Sized> ComposeWithChild<C, ChildKind<K>> for P
where
  P: StateWriter<Value: ComposeChild<'c, Child: RFrom<C, K>>>,
{
  type Target = Pair<P, C>;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { Pair { parent: self, child } }
}

impl<'c, P, C, Builder, K: ?Sized> ComposeWithChild<C, TmlKind<&'c K>> for P
where
  P: StateWriter<Value: ComposeChild<'c, Child: Template<Builder = Builder>>>,
  Builder: Default + ComposeWithChild<C, K>,
{
  type Target = Pair<P, Builder::Target>;

  fn with_child(self, child: C) -> Self::Target {
    Pair { parent: self, child: Builder::default().with_child(child) }
  }
}

pub struct StatelessKind<K: ?Sized>(PhantomData<fn() -> K>);
impl<'c, P, C, K: ?Sized> ComposeWithChild<C, StatelessKind<K>> for P
where
  P: ComposeChild<'c>,
  State<P>: ComposeWithChild<C, K>,
{
  type Target = <State<P> as ComposeWithChild<C, K>>::Target;
  fn with_child(self, child: C) -> Self::Target { State::value(self).with_child(child) }
}

impl<P, C, K: ?Sized> ComposeWithChild<C, K> for FatObj<P>
where
  P: ComposeWithChild<C, K>,
{
  type Target = FatObj<P::Target>;

  #[track_caller]
  fn with_child(self, child: C) -> Self::Target {
    // Employing a verbose method to ensure accurate panic location reporting,
    // since the `closure_track_caller` macro is currently in an unstable state.
    // Once `closure_track_caller` becomes stable, a more concise alternative would
    // be: `self.map(|p| p.with_child(child))`
    let (host, fat) = self.into_parts();
    let child = host.with_child(child);
    fat.map(|_| child)
  }
}

impl<P, C, Child, K: ?Sized> ComposeWithChild<Child, K> for Pair<P, C>
where
  C: ComposeWithChild<Child, K>,
{
  type Target = Pair<P, C::Target>;
  #[inline]
  fn with_child(self, child: Child) -> Self::Target {
    let Pair { parent, child: c } = self;
    Pair { parent, child: c.with_child(child) }
  }
}

pub trait OptionComposeWithChild<'c, C, K: ?Sized> {
  fn with_child(self, child: C) -> Widget<'c>;
}
impl<'c, P, C, K> OptionComposeWithChild<'c, C, K> for Option<P>
where
  P: ComposeWithChild<C, K>,
  C: IntoWidget<'c, K>,
  P::Target: IntoWidget<'c, K>,
{
  #[inline]
  fn with_child(self, child: C) -> Widget<'c> {
    if let Some(p) = self { p.with_child(child).into_widget() } else { child.into_widget() }
  }
}

pub struct BuildTml;
impl<B: TemplateBuilder> RFrom<B, BuildTml> for B::Target {
  #[inline]
  fn r_from(from: B) -> Self { from.build_tml() }
}

// ---- convert to widget -------
impl<'w, P, C, K: ?Sized> RFrom<Pair<P, C>, OtherWidget<K>> for Widget<'w>
where
  P: StateWriter<Value: ComposeChild<'w, Child: RFrom<C, K>>>,
{
  #[inline]
  fn r_from(from: Pair<P, C>) -> Self {
    let Pair { parent, child } = from;
    ComposeChild::compose_child(parent, child.r_into())
  }
}

// impl Option as Template
impl<T> Template for Option<T> {
  type Builder = OptionBuilder<T>;

  #[inline]
  fn builder() -> Self::Builder { OptionBuilder(None) }
}

/// The template builder for `Option` introduces a new type to disambiguate the
/// `with_child` method call for `Option`, especially when `Option` acts as a
/// parent for a widget with `with_child` method.
pub struct OptionBuilder<T>(pub Option<T>);

impl<T> Default for OptionBuilder<T> {
  fn default() -> Self { Self(None) }
}

impl<T> TemplateBuilder for OptionBuilder<T> {
  type Target = Option<T>;
  #[inline]
  fn build_tml(self) -> Self::Target { self.0 }
}

impl<C, T, K: ?Sized> ComposeWithChild<C, ValueKind<K>> for OptionBuilder<T>
where
  C: RInto<T, K>,
{
  type Target = Self;

  fn with_child(self, child: C) -> Self::Target { self.with_child(Some(child)) }
}

impl<C, T, K: ?Sized> ComposeWithChild<Option<C>, Option<fn() -> K>> for OptionBuilder<T>
where
  C: RInto<T, K>,
{
  type Target = Self;

  fn with_child(mut self, child: Option<C>) -> Self::Target {
    debug_assert!(self.0.is_none(), "Option already has a child");
    self.0 = child.map(RInto::r_into);
    self
  }
}

// impl Vec<T> as Template

pub struct VecBuilder<T>(Vec<T>);

impl<T> Default for VecBuilder<T> {
  fn default() -> Self { Self(vec![]) }
}

impl<T> Template for Vec<T> {
  type Builder = VecBuilder<T>;
  #[inline]
  fn builder() -> Self::Builder { VecBuilder(vec![]) }
}

impl<T> TemplateBuilder for VecBuilder<T> {
  type Target = Vec<T>;
  #[inline]
  fn build_tml(self) -> Self::Target { self.0 }
}

impl<C, T, K: ?Sized> ComposeWithChild<C, std::iter::Once<&'static K>> for VecBuilder<T>
where
  C: RInto<T, K>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C) -> Self::Target {
    self.0.push(child.r_into());
    self
  }
}

impl<C, T, K: ?Sized> ComposeWithChild<C, dyn Iterator<Item = K>> for VecBuilder<T>
where
  C: IntoIterator<Item: RInto<T, K>>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C) -> Self::Target {
    self
      .0
      .extend(child.into_iter().map(|v| v.r_into()));
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::{MockBox, MockStack};

  #[derive(Template)]
  enum PTml {
    Void(Void),
  }

  struct P;

  impl ComposeChild<'static> for P {
    type Child = PTml;
    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
      Void.into_widget()
    }
  }

  #[derive(Declare)]
  struct XX;

  impl<'c> ComposeChild<'c> for XX {
    type Child = Widget<'c>;

    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'c> {
      Void.into_widget()
    }
  }

  #[test]
  fn template_fill_template() { let _ = |_: &BuildCtx| P.with_child(Void).into_widget(); }

  #[test]
  fn pair_compose_child() {
    let _ = |_: &BuildCtx| -> Widget {
      MockBox { size: ZERO_SIZE }
        .with_child(XX.with_child(Void {}))
        .into_widget()
    };
  }

  #[derive(Declare)]
  struct PipeParent;

  impl ComposeChild<'static> for PipeParent {
    type Child = BoxPipe<usize>;

    fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
      Void.into_widget()
    }
  }

  #[test]
  fn compose_pipe_child() {
    let _value_child = fn_widget! {
      @PipeParent {  @ { BoxPipe::value(0) } }
    };

    let _pipe_child = fn_widget! {
      let state = State::value(0);
      @PipeParent {  @ { pipe!(*$state) } }
    };
  }

  #[test]
  fn compose_template_enum() {
    #[allow(dead_code)]
    #[derive(Template)]
    enum EnumTml {
      Widget(Widget<'static>),
      Text(TextValue),
    }

    #[derive(Declare)]
    struct EnumTest {}

    impl ComposeChild<'static> for EnumTest {
      type Child = Vec<EnumTml>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
        unreachable!()
      }
    }

    let _ = fn_widget! {
      let v = Stateful::new(true);
      let w = EnumTml::Widget(fn_widget! { @Void {} }.into_widget());
      @EnumTest {
        @ Void {}
        @ { "test" }
        @ { pipe!(*$v).map(|_| fn_widget! { @Void {} }) }
        @ MockStack { @Void {} }
        @ {w}
      }
    };
  }

  #[test]
  fn enum_conversion() {
    pub struct BuilderX;

    struct BuilderAKind<K: ?Sized>(PhantomData<fn() -> K>);
    struct BuilderBKind<K: ?Sized>(PhantomData<fn() -> K>);

    impl<C, K: ?Sized> RFrom<C, BuilderAKind<K>> for BuilderX
    where
      C: RInto<Widget<'static>, K>,
    {
      fn r_from(_: C) -> Self { unreachable!() }
    }

    impl<C, K: ?Sized> RFrom<C, BuilderBKind<K>> for BuilderX
    where
      C: RInto<CowArc<str>, K>,
    {
      fn r_from(_: C) -> Self { unreachable!() }
    }

    impl<'w, K: ?Sized, C> ComposeWithChild<C, BuilderAKind<K>> for BuilderX
    where
      C: RInto<Widget<'w>, K>,
    {
      type Target = Self;
      fn with_child(self, _: C) -> Self { unreachable!() }
    }

    impl<K: ?Sized, C> ComposeWithChild<C, BuilderBKind<K>> for BuilderX
    where
      C: RInto<CowArc<str>, K>,
    {
      type Target = Self;
      fn with_child(self, _: C) -> Self { unreachable!() }
    }
    let _ = move || {
      let builder = BuilderX;
      let builder = builder.with_child("Hello");
      let _builder = builder.with_child(Void);
      let _builder: BuilderX = "hello".r_into();
      let _builder: BuilderX = Void.r_into();
    };
  }
}
