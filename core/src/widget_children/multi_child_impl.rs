use super::*;
use crate::pipe::InnerPipe;

pub struct MultiPair<'a> {
  pub parent: Widget<'static>,
  pub children: Vec<Widget<'a>>,
}

pub trait MultiIntoParent: 'static {
  fn into_parent(self) -> Widget<'static>;
}

macro_rules! impl_widget_child {
  ($($m: ident),*) => {
    $(
      // Choose `IntoWidgetStrict` for child widgets instead of `IntoWidget`. This is
      // because `IntoWidget` may lead
      // `Pipe<Value = Option<impl IntoWidget>>` has two implementations:
      //
      // - As a single widget child, satisfy the `IntoWidget` requirement, albeit not
      //   `IntoWidget`.
      // - As a `Pipe` that facilitates iteration over multiple widgets.
      impl<'w, 'c:'w, C> WithChild<'w, C, 1, { 100 + $m }> for MultiPair<'w>
      where
        C: IntoWidgetStrict<'c, $m>
      {
        type Target = MultiPair<'w>;
        #[inline]
        fn with_child(mut self, child: C) -> Self::Target {
          self.children.push(child.into_widget_strict());
          self
        }
      }
    )*
  };
}

macro_rules! impl_iter_widget_child {
  ($($m: ident), *) => {
    $(
      impl<'w, 'v: 'w, C> WithChild<'w, C, 1, { 110 + $m }> for MultiPair<'w>
      where
        C:IntoIterator,
        C::Item: IntoWidget<'v, $m>,
      {
        type Target = MultiPair<'w>;
        #[inline]
        fn with_child(mut self, child: C) -> Self::Target {
          self.children.extend(child.into_iter().map(|w| w.into_widget()));
          self
        }
      }
    )*
  };
}

macro_rules! impl_pipe_iter_widget_child {
  ($($m: ident), *) => {
    $(
      impl<'w, C> WithChild<'w, C, 1, { 120 + $m }> for MultiPair<'w>
      where
        C:InnerPipe,
        C::Value: IntoIterator,
        <C::Value as IntoIterator>::Item: IntoWidget<'static, $m>,
      {
        type Target = MultiPair<'w>;

        fn with_child(mut self, child: C) -> Self::Target {
          self.children.extend(child.build_multi());
          self
        }
      }
    )*
  };
}

impl_widget_child!(COMPOSE, RENDER, FN);
impl_iter_widget_child!(COMPOSE, RENDER, FN);
impl_pipe_iter_widget_child!(COMPOSE, RENDER, FN);

impl<'w, C, T, const M: usize> WithChild<'w, C, 1, M> for T
where
  T: MultiIntoParent,
  MultiPair<'w>: WithChild<'w, C, 1, M>,
{
  type Target = <MultiPair<'w> as WithChild<'w, C, 1, M>>::Target;

  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    MultiPair { parent: self.into_parent(), children: vec![] }.with_child(child)
  }
}

impl<'w> WithChild<'w, Widget<'w>, 1, FN> for MultiPair<'w> {
  type Target = MultiPair<'w>;

  #[inline]
  fn with_child(mut self, child: Widget<'w>) -> Self::Target {
    self.children.push(child);
    self
  }
}

impl<'w> IntoWidgetStrict<'w, FN> for MultiPair<'w> {
  fn into_widget_strict(self) -> Widget<'w> {
    let f = move |ctx: &mut BuildCtx| {
      let MultiPair { parent, children } = self;
      parent.directly_compose_children(children)
    };

    f.into_widget()
  }
}

// Implementation `IntoParent`
impl<P: MultiChild + IntoWidget<'static, RENDER>> MultiIntoParent for P {
  #[inline]
  fn into_parent(self) -> Widget<'static> { self.into_widget() }
}

impl<S, V, F> MultiIntoParent for MapPipe<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: MultiIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { self.into_parent_widget() }
}

impl<S, V, F> MultiIntoParent for FinalChain<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: MultiIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { self.into_parent_widget() }
}

impl<V> MultiIntoParent for Box<dyn Pipe<Value = V>>
where
  V: MultiIntoParent + IntoWidget<'static, RENDER>,
{
  fn into_parent(self) -> Widget<'static> { self.into_parent_widget() }
}

impl<P: MultiIntoParent> MultiIntoParent for FatObj<P> {
  fn into_parent(self) -> Widget<'static> { self.map(|p| p.into_parent()).into_widget() }
}
