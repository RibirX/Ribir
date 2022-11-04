use crate::impl_proxy_query;
pub use crate::prelude::*;
use std::marker::PhantomData;

/// A generic widget wrap for all compose widget result, and keep its type info.
pub(crate) struct ComposedWidget<R, B> {
  composed: R,
  by: PhantomData<B>,
}
impl<B> ComposedWidget<Widget, B> {
  #[inline]
  pub fn new(composed: Widget) -> Self { ComposedWidget { composed, by: PhantomData } }
}

impl<B: 'static> IntoWidget<Widget> for ComposedWidget<Widget, B> {
  fn into_widget(self) -> Widget {
    let Widget { node, mut children } = self.composed;
    let by = self.by;
    if let Some(node) = node {
      match node {
        WidgetNode::Compose(c) => {
          assert!(children.is_empty());
          (move |ctx: &BuildCtx| ComposedWidget { composed: c(ctx), by }.into_widget())
            .into_widget()
        }
        WidgetNode::Render(r) => {
          let node = WidgetNode::Render(Box::new(ComposedWidget { composed: r, by }));
          Widget { node: Some(node), children }
        }
      }
    } else {
      match children.len() {
        0 => Widget { node: None, children },
        1 => Self {
          composed: children.pop().unwrap(),
          by,
        }
        .into_widget(),
        _ => unreachable!("Compose return multi widget, should compile failed."),
      }
    }
  }
}

impl<B: 'static> Render for ComposedWidget<Box<dyn Render>, B> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.composed.perform_layout(clamp, ctx)
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.composed.paint(ctx) }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.composed.only_sized_by_parent() }

  #[inline]
  fn can_overflow(&self) -> bool { self.composed.can_overflow() }

  #[inline]
  fn hit_test(&self, ctx: &TreeCtx, pos: Point) -> HitTest { self.composed.hit_test(ctx, pos) }
}

impl<W: SingleChild, B> SingleChild for ComposedWidget<W, B> {}

impl<W: MultiChild, B> MultiChild for ComposedWidget<W, B> {}

impl<B: 'static> Query for ComposedWidget<Box<dyn Render>, B>
where
  Self: Render + 'static,
{
  impl_proxy_query!(self.composed);
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn fix_compose_expr_crash() {
    #[derive(Debug)]
    struct T;

    impl Compose for T {
      fn compose(this: StateWidget<Self>) -> Widget {
        widget! {
          track { this: this.into_stateful() }
          DynWidget {
            dyns: {
               // explicit capture `this` to avoid `DynWidget` to be optimized`.
              let x = &*this;
              println!("{:?}", x);
              Void
            },
          }
        }
      }
    }

    let _ = widget_and_its_children_box_rect(T.into_widget(), Size::zero());
  }
}
