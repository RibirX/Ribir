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
    let Widget { node, children } = self.composed;
    let by = self.by;
    if let Some(node) = node {
      match node {
        WidgetNode::Compose(c) => {
          assert!(children.is_none());
          let composed =
            { move |ctx: &mut BuildCtx| ComposedWidget { composed: c(ctx), by }.into_widget() };
          composed.into_widget()
        }
        WidgetNode::Render(r) => {
          let node = WidgetNode::Render(Box::new(ComposedWidget { composed: r, by }));
          Widget { node: Some(node), children }
        }
        WidgetNode::Dynamic(ExprWidget { mut expr, upstream }) => {
          let new_expr = move |ctx: &mut BuildCtx| match expr(ctx) {
            DynamicWidget::Single(w) => {
              DynamicWidget::Single(w.map(|w| ComposedWidget { composed: w, by }.into_widget()))
            }
            DynamicWidget::Multi(_) => {
              unreachable!("`ExprWidget` from compose widget, must be generate single child.")
            }
          };
          Widget {
            node: Some(WidgetNode::Dynamic(ExprWidget {
              expr: Box::new(new_expr),
              upstream,
            })),
            children,
          }
        }
      }
    } else {
      match children {
        Children::None => Widget { node: None, children: Children::None },
        Children::Single(s) => Self { composed: *s, by }.into_widget(),
        Children::Multi(_) => unreachable!("Compose return multi widget, should compile failed."),
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
  fn only_sized_by_parent(&self) -> bool { self.composed.only_sized_by_parent() }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.composed.paint(ctx) }
}

impl<W: SingleChild, B> SingleChild for ComposedWidget<W, B> {}

impl<W: MultiChild, B> MultiChild for ComposedWidget<W, B> {}

impl<B: 'static> Query for ComposedWidget<Box<dyn Render>, B>
where
  Self: Render + 'static,
{
  impl_proxy_query!(composed);
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
          ExprWidget {
            expr: {
               // explicit capture `this` to avoid `ExprWidget` to be optimized`.
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
