use super::*;

/// The trait is used to enable child composition for `ComposeChild`.
///
/// We choose to return a pair of parent and child instead of directly composing
/// and returning a `Widget`. This approach allows for continued composition
/// with certain child types like `Vec`.
///
/// The `M` and `WRITER` use to avoid implementation conflicts.
///
/// - The `WRITER` marker if it is a writer.
/// - The `TML` marker is used for `Template` to indicate it will convert by
///   TML's level of 'Template', 0 for no 'Template'
/// - The `N` marker is used to distinguish the type fill in `Template`
/// - The `M` marker is used for child conversion.
pub trait ComposeWithChild<
  'w,
  C,
  const WRITER: bool,
  const TML: usize,
  const N: usize,
  const M: usize,
>
{
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

// ComposeWithChild implementations
impl<'w, P, C, const M: usize> ComposeWithChild<'w, C, true, 0, 0, M> for P
where
  P: StateWriter,
  P::Value: ComposeChild<'w>,
  C: IntoChildCompose<<P::Value as ComposeChild<'w>>::Child, M>,
{
  type Target = Pair<P, <P::Value as ComposeChild<'w>>::Child>;

  fn with_child(self, child: C) -> Self::Target {
    Pair { parent: self, child: child.into_child_compose() }
  }
}

impl<'w, P, Builder, C, const N: usize, const M: usize> ComposeWithChild<'w, C, true, 1, N, M> for P
where
  P: StateWriter,
  P::Value: ComposeChild<'w, Child: Template<Builder = Builder>>,
  Builder: ComposeWithChild<'w, C, false, 1, N, M>,
{
  type Target = Pair<Self, Builder::Target>;

  fn with_child(self, child: C) -> Self::Target {
    let child = <P::Value as ComposeChild<'w>>::Child::builder().with_child(child);
    Pair { parent: self, child }
  }
}

impl<'w, P, Builder, C, const N: usize, const M: usize> ComposeWithChild<'w, C, true, 2, N, M> for P
where
  P: StateWriter,
  P::Value: ComposeChild<'w, Child: Template<Builder = Builder>>,
  Builder: ComposeWithChild<'w, C, false, 2, N, M>,
{
  type Target = Pair<Self, Builder::Target>;

  fn with_child(self, child: C) -> Self::Target {
    let child = <P::Value as ComposeChild<'w>>::Child::builder().with_child(child);
    Pair { parent: self, child }
  }
}

impl<'w, P, C, Target, const TML: usize, const N: usize, const M: usize>
  ComposeWithChild<'w, C, false, TML, N, M> for P
where
  P: ComposeChild<'w>,
  State<P>: ComposeWithChild<'w, C, true, TML, N, M, Target = Target>,
{
  type Target = Target;

  fn with_child(self, child: C) -> Self::Target { State::value(self).with_child(child) }
}

impl<'w, W, C, const TML: usize, const WRITER: bool, const N: usize, const M: usize>
  ComposeWithChild<'w, C, WRITER, TML, N, M> for FatObj<W>
where
  W: ComposeWithChild<'w, C, WRITER, TML, N, M>,
{
  type Target = FatObj<W::Target>;

  fn with_child(self, child: C) -> Self::Target { self.map(|host| host.with_child(child)) }
}

// Option needn't implement for `Template`
impl<'w, T, C, const WRITER: bool, const N: usize, const M: usize>
  ComposeWithChild<'w, C, WRITER, 0, N, M> for Option<T>
where
  T: ComposeWithChild<'w, C, WRITER, 0, 0, M>,
  C: IntoChildCompose<Widget<'w>, M>,
  T::Target: IntoWidget<'w, N>,
{
  type Target = Widget<'w>;

  fn with_child(self, c: C) -> Self::Target {
    if let Some(p) = self { p.with_child(c).into_widget() } else { c.into_child_compose() }
  }
}

// The continuation with a child is only possible if the child of `Pair` is a
// `Template`.
impl<'w, W, C1, C2: 'w, const WRITER: bool, const TML: usize, const M: usize, const N: usize>
  ComposeWithChild<'w, C2, WRITER, TML, N, M> for Pair<W, C1>
where
  C1: ComposeWithChild<'w, C2, WRITER, TML, N, M>,
{
  type Target = Pair<W, C1::Target>;

  fn with_child(self, c: C2) -> Self::Target {
    let Pair { parent: widget, child } = self;
    Pair { parent: widget, child: child.with_child(c) }
  }
}

impl<'w, W, C: 'w> IntoWidget<'w, COMPOSE> for Pair<W, C>
where
  W: StateWriter,
  W::Value: ComposeChild<'w, Child = C>,
{
  fn into_widget(self) -> Widget<'w> {
    let Self { parent, child } = self;
    ComposeChild::compose_child(parent, child).into_widget()
  }
}

impl<'w, W, C, TML> IntoWidget<'w, FN> for Pair<W, C>
where
  W: StateWriter,
  W::Value: ComposeChild<'w, Child = TML>,
  TML: Template<Builder = C>,
  C: TemplateBuilder<Target = TML> + 'w,
{
  fn into_widget(self) -> Widget<'w> {
    let Self { parent, child } = self;
    ComposeChild::compose_child(parent, child.build_tml()).into_widget()
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
pub struct OptionBuilder<T>(Option<T>);

impl<T> TemplateBuilder for OptionBuilder<T> {
  type Target = Option<T>;
  #[inline]
  fn build_tml(self) -> Self::Target { self.0 }
}

impl<T> ComposeChildFrom<OptionBuilder<T>, 1> for Option<T> {
  #[inline]
  fn compose_child_from(from: OptionBuilder<T>) -> Self { from.build_tml() }
}

impl<'w, C, T, const M: usize> ComposeWithChild<'w, C, false, 1, 0, M> for OptionBuilder<T>
where
  C: IntoChildCompose<T, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { self.with_child(Some(child)) }
}

impl<'w, C, T, const M: usize> ComposeWithChild<'w, Option<C>, false, 1, 1, M> for OptionBuilder<T>
where
  C: IntoChildCompose<T, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: Option<C>) -> Self::Target {
    debug_assert!(self.0.is_none(), "Option already has a child");
    self.0 = child.map(IntoChildCompose::into_child_compose);
    self
  }
}

// impl Vec<T> as Template

pub struct VecBuilder<T>(Vec<T>);

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

impl<T> ComposeChildFrom<VecBuilder<T>, 1> for Vec<T> {
  #[inline]
  fn compose_child_from(from: VecBuilder<T>) -> Self { from.build_tml() }
}

impl<'w, C, T, const M: usize> ComposeWithChild<'w, C, false, 1, 0, M> for VecBuilder<T>
where
  C: IntoChildCompose<T, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C) -> Self::Target {
    self.0.push(child.into_child_compose());
    self
  }
}

impl<'w, C, T, const N: usize, const M: usize> ComposeWithChild<'w, C, false, 2, N, M>
  for VecBuilder<T>
where
  T: Template,
  T::Builder: ComposeWithChild<'w, C, false, 1, N, M>,
  <T::Builder as ComposeWithChild<'w, C, false, 1, N, M>>::Target: TemplateBuilder<Target = T>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C) -> Self::Target {
    self
      .0
      .push(T::builder().with_child(child).build_tml());
    self
  }
}

impl<'w, C, T, const M: usize> ComposeWithChild<'w, C, false, 1, 1, M> for VecBuilder<T>
where
  C: IntoIterator,
  C::Item: IntoChildCompose<T, M>,
{
  type Target = Self;

  #[inline]
  fn with_child(mut self, child: C) -> Self::Target {
    self
      .0
      .extend(child.into_iter().map(|v| v.into_child_compose()));
    self
  }
}

// todo: remove it, keep it for backward compatibility.

impl ChildOfCompose for Resource<PixelImage> {}

pub trait CompatibilityWithChild<'w, C, const N: usize, const M: usize> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

impl<'w, W, C, const M: usize> CompatibilityWithChild<'w, C, 8, M> for State<W>
where
  W: ComposeDecorator + 'static,
  C: IntoWidget<'w, M>,
{
  type Target = Widget<'w>;

  fn with_child(self, child: C) -> Self::Target {
    let host = child.into_widget();

    let f = move || {
      let tid = TypeId::of::<W>();
      let ctx = BuildCtx::get();
      let decor = Provider::of::<ComposeDecorators>(BuildCtx::get())
        .and_then(|t| QueryRef::filter_map(t, |t| t.styles.get(&tid)).ok());

      if let Some(style) = decor {
        style(Box::new(self), host, ctx)
      } else {
        ComposeDecorator::compose_decorator(self, host).into_widget()
      }
    };
    f.into_widget()
  }
}

impl<'w, T, C, const M: usize> CompatibilityWithChild<'w, C, 9, M> for T
where
  T: ComposeDecorator + 'static,
  C: IntoWidget<'w, M>,
{
  type Target = Widget<'w>;

  fn with_child(self, child: C) -> Self::Target { State::value(self).with_child(child) }
}

impl<'w, W, C, const N: usize, const M: usize> CompatibilityWithChild<'w, C, N, M> for FatObj<W>
where
  W: CompatibilityWithChild<'w, C, N, M>,
{
  type Target = FatObj<W::Target>;

  fn with_child(self, child: C) -> Self::Target { self.map(|host| host.with_child(child)) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::{MockBox, MockStack};

  #[derive(Template)]
  enum PTml {
    Void(Void),
  }

  impl ChildOfCompose for Void {}

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
      Text(TextInit),
    }

    #[derive(Declare)]
    struct EnumTest {}

    impl ComposeChild<'static> for EnumTest {
      type Child = Vec<EnumTml>;

      fn compose_child(_: impl StateWriter<Value = Self>, _: Self::Child) -> Widget<'static> {
        todo!()
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
}
