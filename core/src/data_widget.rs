//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use crate::{
  impl_proxy_query, impl_proxy_render, impl_query_self_only, prelude::*, widget::FnWidget,
};

pub struct DataWidget<D> {
  render: Box<dyn Render>,
  data: D,
}

impl<D: Query + 'static> DataWidget<D> {
  pub fn new(render: Box<dyn Render>, data: D) -> Self { DataWidget { render, data } }

  pub fn attach(widget: Widget, data: D) -> Widget {
    FnWidget::new(move |ctx| {
      let id = widget.build(ctx);
      attach_to_id(id, ctx.force_as_mut(), |child| {
        Box::new(Self::new(child, data))
      });
      id
    })
    .into()
  }

  pub fn attach_state(widget: Widget, data: State<D>) -> Widget {
    match data {
      State::Stateless(data) => DataWidget::attach(widget, data),
      State::Stateful(data) => DataWidget::attach(widget, data),
    }
  }
}

pub fn attach_to_id(
  id: WidgetId,
  ctx: &mut BuildCtx,
  attach_data: impl FnOnce(Box<dyn Render>) -> Box<dyn Render>,
) {
  let mut tmp: Box<dyn Render> = Box::new(Void);
  let node = ctx.assert_get_mut(id);
  std::mem::swap(&mut tmp, node);

  let mut attached = attach_data(tmp);
  std::mem::swap(node, &mut attached);
}

impl_proxy_query!(paths [data, render], DataWidget<D>, <D>, where D: Query + 'static);
impl_proxy_render!(proxy render, DataWidget<D>, <D>, where D: Query + 'static);

/// Data attach widget that we don't care about its type.
pub struct AnonymousData(Box<dyn Any>);

impl AnonymousData {
  #[inline]
  pub fn new(data: Box<dyn Any>) -> Self { Self(data) }
}

impl_query_self_only!(AnonymousData);
impl_query_self_only!(Vec<AnonymousData>);
