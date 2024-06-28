use super::*;
use crate::pipe::InnerPipe;

// We preserve the parent type, while the child must always be a widget. This is
// important to maintain the integrity of the template, as it may rely on
// accessing the parent type.
impl<P, C, const M: usize> WithChild<C, 0, M> for P
where
  P: Parent,
  C: IntoWidget<M>,
{
  type Target = WidgetOf<Self>;
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    Pair { parent: self, child: child.into_widget(ctx) }
  }
}

// with option child
impl<P, C, const M: usize> WithChild<Option<C>, 0, M> for P
where
  P: SingleIntoParent,
  C: IntoWidget<M>,
{
  type Target = Widget;
  #[track_caller]
  fn with_child(self, child: Option<C>, ctx: &BuildCtx) -> Widget {
    if let Some(child) = child {
      Pair { parent: self, child: child.into_widget(ctx) }.into_widget(ctx)
    } else {
      self.into_parent(ctx)
    }
  }
}

// option parent with child
impl<P, C, const M: usize> WithChild<C, 0, M> for Option<P>
where
  P: SingleIntoParent,
  C: IntoWidget<M>,
{
  type Target = Widget;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Widget {
    if let Some(parent) = self {
      Pair { parent, child: child.into_widget(ctx) }.into_widget(ctx)
    } else {
      child.into_widget(ctx)
    }
  }
}

impl<W: SingleIntoParent> IntoWidgetStrict<RENDER> for WidgetOf<W> {
  fn into_widget_strict(self, ctx: &BuildCtx) -> Widget {
    let Pair { parent, child } = self;
    let p = parent.into_parent(ctx);
    let p_leaf = p.id().single_leaf(&ctx.tree.borrow().arena);
    ctx.append_child(p_leaf, child);
    p
  }
}

/// This trait indicates that a type can serve as a parent widget for another
/// widget. It is similar to the `SingleChild` trait but includes the concept of
/// a pipe for `SingleChild`. The reason for this distinction is that the logic
/// for being a parent widget with a pipe differs from that of a regular render
/// widget. Therefore, the pipe does not implement the `SingleChild` trait but
/// instead utilizes the `Parent` trait to distinguish between the two.

trait Parent {}

impl<T: SingleChild> Parent for T {}
impl<T: Parent> Parent for FatObj<T> {}

impl<S, V, F> Parent for MapPipe<V, S, F>
where
  S: Pipe,
  V: Parent,
  F: FnMut(S::Value) -> V,
{
}

impl<S, V, F> Parent for FinalChain<V, S, F>
where
  S: Pipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V>,
  V: Parent,
{
}

impl<V: Parent> Parent for Box<dyn Pipe<Value = V>> {}

impl<S, V, F> Parent for MapPipe<Option<V>, S, F>
where
  S: Pipe,
  V: Parent,
  F: FnMut(S::Value) -> Option<V>,
{
}

impl<S, V, F> Parent for FinalChain<Option<V>, S, F>
where
  S: Pipe<Value = Option<V>>,
  F: FnOnce(ValueStream<Option<V>>) -> ValueStream<Option<V>>,
  V: Parent,
{
}

impl<V: Parent> Parent for Box<dyn Pipe<Value = Option<V>>> {}

/// This trait converts a parent into a widget. We require this trait because
/// the logic for being a parent widget with a pipe differs from that of a
/// regular render widget.
pub trait SingleIntoParent {
  fn into_parent(self, ctx: &BuildCtx) -> Widget;
}

// Implementation `IntoParent`
impl<P: SingleChild + IntoWidget<RENDER>> SingleIntoParent for P {
  #[inline]
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_widget(ctx) }
}

impl<P: SingleIntoParent> SingleIntoParent for FatObj<P> {
  fn into_parent(self, ctx: &BuildCtx) -> Widget {
    self.map(|p| p.into_parent(ctx)).into_widget(ctx)
  }
}

impl<S, V, F> SingleIntoParent for MapPipe<V, S, F>
where
  S: Pipe,
  V: SingleIntoParent + IntoWidget<RENDER> + 'static,
  S::Value: 'static,
  F: FnMut(S::Value) -> V + 'static,
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_parent_widget(ctx) }
}

impl<S, V, F> SingleIntoParent for FinalChain<V, S, F>
where
  S: Pipe<Value = V>,
  F: FnOnce(ValueStream<V>) -> ValueStream<V> + 'static,
  V: SingleIntoParent + IntoWidget<RENDER> + 'static,
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_parent_widget(ctx) }
}

impl<V: SingleIntoParent + IntoWidget<RENDER> + 'static> SingleIntoParent
  for Box<dyn Pipe<Value = V>>
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { self.into_parent_widget(ctx) }
}

impl<S, V, F> SingleIntoParent for MapPipe<Option<V>, S, F>
where
  S: Pipe,
  V: SingleIntoParent + IntoWidget<RENDER> + 'static,
  F: FnMut(S::Value) -> Option<V> + 'static,
  S::Value: 'static,
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { pipe_option_parent(self, ctx) }
}

impl<S, V, F> SingleIntoParent for FinalChain<Option<V>, S, F>
where
  S: Pipe<Value = Option<V>>,
  F: FnOnce(ValueStream<Option<V>>) -> ValueStream<Option<V>> + 'static,
  V: SingleIntoParent + IntoWidget<RENDER> + 'static,
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { pipe_option_parent(self, ctx) }
}

impl<V: SingleIntoParent + IntoWidget<RENDER> + 'static> SingleIntoParent
  for Box<dyn Pipe<Value = Option<V>>>
{
  fn into_parent(self, ctx: &BuildCtx) -> Widget { pipe_option_parent(self, ctx) }
}

fn pipe_option_parent(
  p: impl InnerPipe<Value = Option<impl IntoWidget<RENDER> + 'static>>, ctx: &BuildCtx,
) -> Widget {
  p.map(|w| {
    move |ctx: &BuildCtx| {
      if let Some(w) = w { w.into_widget(ctx) } else { Void.into_widget(ctx) }
    }
  })
  .into_parent_widget(ctx)
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::MockBox;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = |ctx| -> Widget {
      mock_box
        .clone()
        .with_child(mock_box.clone().with_child(mock_box, ctx), ctx)
        .into_widget(ctx)
    };
  }

  #[test]
  fn fix_mock_box_compose_pipe_option_widget() {
    fn _x(w: BoxPipe<Option<Widget>>, ctx: &BuildCtx) {
      MockBox { size: ZERO_SIZE }.with_child(w.into_pipe(), ctx);
    }
  }
}
