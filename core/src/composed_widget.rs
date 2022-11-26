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

impl<B: 'static> IntoWidget<Generic<Widget>> for ComposedWidget<Widget, B> {
  fn into_widget(self) -> Widget {
    let by = self.by;

    match self.composed {
      Widget::Compose(c) => {
        (move |ctx: &BuildCtx| ComposedWidget { composed: c(ctx), by }.into_widget()).into_widget()
      }
      Widget::Render { render, children } => Widget::Render {
        render: Box::new(ComposedWidget { composed: render, by }),
        children,
      },
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
  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest { self.composed.hit_test(ctx, pos) }
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

    let mut wnd = Window::default_mock(T.into_widget(), None);
    wnd.draw_frame();
  }
}
