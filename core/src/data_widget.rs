//! Data widget help attach data to a widget and get a new widget witch behavior
//! is same as origin widget.

use crate::{impl_proxy_query, impl_proxy_render, impl_query_self_only, prelude::*};

pub struct DataWidget<W, D> {
  widget: W,
  data: D,
}

impl<W, D> DataWidget<W, D> {
  #[inline]
  pub fn new(widget: W, data: D) -> Self { Self { widget, data } }
}

impl<W: Render + 'static, D: Query + 'static> Render for DataWidget<W, D> {
  impl_proxy_render!(widget);
}

impl<W: Query + 'static, D: Query + 'static> Query for DataWidget<W, D> {
  impl_proxy_query!(self.data, self.widget);
}

pub fn compose_child_as_data_widget<D: Query + 'static>(
  child: Widget,
  data: StateWidget<D>,
) -> Widget {
  match data {
    StateWidget::Stateless(data) => widget_attach_data(child, data),
    StateWidget::Stateful(data) => widget_attach_data(child, data),
  }
}

pub fn widget_attach_data<D: Query + 'static>(widget: Widget, data: D) -> Widget {
  match widget {
    Widget::Compose(c) => (|ctx: &BuildCtx| widget_attach_data(c(ctx), data)).into_widget(),
    Widget::Render { render, children } => Widget::Render {
      render: Box::new(DataWidget { widget: render, data }),
      children,
    },
  }
}

/// Data attach widget that we don't care about its type.
pub struct AnonymousData(Box<dyn Any>);

impl AnonymousData {
  #[inline]
  pub fn new(data: Box<dyn Any>) -> Self { Self(data) }
}

impl Query for AnonymousData {
  impl_query_self_only!();
}

impl Query for Vec<AnonymousData> {
  impl_query_self_only!();
}
