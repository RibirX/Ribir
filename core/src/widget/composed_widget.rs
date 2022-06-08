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
    let by = self.by;
    match self.composed.0 {
      WidgetInner::Compose(c) => {
        { move |ctx: &mut BuildCtx| ComposedWidget { composed: c(ctx), by }.into_widget() }
          .into_widget()
      }
      WidgetInner::Render(r) => ComposedWidget { composed: r, by }.into_widget(),
      WidgetInner::SingleChild(s) => {
        let widget: Box<dyn Render> = Box::new(ComposedWidget { composed: s.widget, by });
        let single = Box::new(SingleChildWidget { widget, child: s.child });
        Widget(WidgetInner::SingleChild(single))
      }
      WidgetInner::MultiChild(m) => {
        let widget: Box<dyn Render> = Box::new(ComposedWidget { composed: m.widget, by });
        let multi = MultiChildWidget { widget, children: m.children };
        Widget(WidgetInner::MultiChild(multi))
      }
      WidgetInner::Expr(_) => unreachable!(),
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
