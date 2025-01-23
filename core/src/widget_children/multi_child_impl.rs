use super::*;
use crate::pipe::InnerPipe;

pub struct MultiPair<'a> {
  pub(crate) parent: Widget<'static>,
  pub(crate) children: Vec<Widget<'a>>,
}

impl<'a> MultiPair<'a> {
  #[inline]
  pub fn new<const N: usize, const M: usize>(
    parent: impl MultiChild, children: impl IntoChildMulti<'a, N, M>,
  ) -> Self {
    let children = children.into_child_multi().collect();
    Self { parent: parent.into_widget(), children }
  }

  pub fn with_child<'b, 'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'b, N, M>,
  ) -> MultiPair<'c>
  where
    'a: 'c,
    'b: 'c,
  {
    let mut children: Vec<Widget<'c>> = self.children;
    for c in child.into_child_multi() {
      children.push(c);
    }

    MultiPair { parent: self.parent, children }
  }
}

impl<'c> IntoChildMulti<'c, 0, 0> for Widget<'c> {
  fn into_child_multi(self) -> impl Iterator<Item = Widget<'c>> { std::iter::once(self) }
}

// Choose `IntoWidgetStrict` for child widgets instead of `IntoWidget`. This is
// because `IntoWidget` may lead
// `Pipe<Value = Option<impl IntoWidget>>` has two implementations:
//
// - As a single widget child, satisfy the `IntoWidget` requirement, albeit not
//   `IntoWidget`.
// - As a `Pipe` that facilitates iteration over multiple widgets.
impl<'w, const M: usize, W: IntoWidgetStrict<'w, M>> IntoChildMulti<'w, 0, M> for W {
  fn into_child_multi(self) -> impl Iterator<Item = Widget<'w>> {
    std::iter::once(self.into_widget_strict())
  }
}

impl<'w, I, const M: usize> IntoChildMulti<'w, 1, M> for I
where
  I: IntoIterator + 'w,
  I::Item: IntoWidget<'w, M>,
{
  fn into_child_multi(self) -> impl Iterator<Item = Widget<'w>> {
    self.into_iter().map(|w| w.into_widget())
  }
}

impl<'w, const M: usize, C> IntoChildMulti<'w, 2, M> for C
where
  C: InnerPipe,
  C::Value: IntoIterator,
  <C::Value as IntoIterator>::Item: IntoWidget<'static, M>,
{
  fn into_child_multi(self) -> impl Iterator<Item = Widget<'w>> { self.build_multi().into_iter() }
}

impl<T> MultiChild for T
where
  T: StateReader<Value: MultiChild> + IntoWidget<'static, RENDER>,
{
  type Target<'c> = MultiPair<'c>;
  fn with_child<'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'c, N, M>,
  ) -> MultiPair<'c> {
    MultiPair::new(self, child)
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> { (*self).into_widget() }
}

macro_rules! impl_pipe_methods {
  () => {
    type Target<'c> = MultiPair<'c>;

    fn with_child<'c, const N: usize, const M: usize>(
      self, child: impl IntoChildMulti<'c, N, M>,
    ) -> MultiPair<'c> {
      MultiPair { parent: self.into_parent_widget(), children: child.into_child_multi().collect() }
    }

    fn into_parent(self: Box<Self>) -> Widget<'static> { self.into_parent_widget() }
  };
}

impl<S, V, F> MultiChild for MapPipe<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: MultiChild,
{
  impl_pipe_methods!();
}

impl<S, V, F> MultiChild for FinalChain<V, S, F>
where
  Self: InnerPipe<Value = V>,
  V: MultiChild,
{
  impl_pipe_methods!();
}

impl<V> MultiChild for Box<dyn Pipe<Value = V>>
where
  V: MultiChild,
{
  impl_pipe_methods!();
}

impl<P: MultiChild> MultiChild for FatObj<P> {
  type Target<'c> = FatObj<MultiPair<'c>>;
  fn with_child<'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'c, N, M>,
  ) -> FatObj<MultiPair<'c>> {
    self.map(move |p| MultiPair::new(p, child))
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> {
    let this = *self;
    if !this.has_class() {
      this.into_widget()
    } else {
      panic!("A FatObj should not have a class attribute when acting as a single parent")
    }
  }
}

impl<'a> FatObj<MultiPair<'a>> {
  pub fn with_child<'b, 'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'b, N, M>,
  ) -> FatObj<MultiPair<'c>>
  where
    'a: 'c,
    'b: 'c,
  {
    self.map(move |p| p.with_child(child))
  }
}

impl MultiChild for Box<dyn MultiChild> {
  type Target<'c> = MultiPair<'c>;

  fn with_child<'c, const N: usize, const M: usize>(
    self, child: impl IntoChildMulti<'c, N, M>,
  ) -> MultiPair<'c> {
    MultiPair::new(self, child)
  }

  fn into_parent(self: Box<Self>) -> Widget<'static> { todo!() }
}

impl<'w> IntoWidgetStrict<'w, RENDER> for MultiPair<'w> {
  fn into_widget_strict(self) -> Widget<'w> {
    let MultiPair { parent, children } = self;
    Widget::new(parent, children)
  }
}
