//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use crate::{
  prelude::*,
  render_helper::{RenderProxy, RenderTarget},
};

pub struct DataWidget<D> {
  render: Box<dyn Render>,
  data: D,
}

/// A wrapper widget which can attach any data to a widget and not care about
/// what the data is.
pub struct AnonymousWrapper {
  render: Box<dyn Render>,
  _data: Box<dyn Any>,
}

impl<D: Query> DataWidget<D> {
  pub(crate) fn attach(render: Box<dyn Render>, data: D) -> Box<dyn Render> {
    Box::new(RenderProxy::new(DataWidget { render, data }))
  }
}

impl AnonymousWrapper {
  #[inline]
  pub fn new(render: Box<dyn Render>, data: Box<dyn Any>) -> Self {
    AnonymousWrapper { render, _data: data }
  }
}

impl Widget {
  pub fn attach_data<D: Query>(self, data: D, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().attach_data(data, arena);

    self
  }

  pub fn attach_state_data<D: Query>(
    self, data: impl StateReader<Value = D>, ctx: &BuildCtx,
  ) -> Widget {
    match data.try_into_value() {
      Ok(data) => self.attach_data(data, ctx),
      Err(data) => self.attach_data(data, ctx),
    }
  }

  pub fn attach_anonymous_data(self, data: impl Any, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().attach_anonymous_data(data, arena);
    self
  }
}

impl<D> RenderTarget for DataWidget<D> {
  type Target = dyn Render;

  #[inline]
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(&*self.render) }
}

impl<D: Query> Query for DataWidget<D> {
  fn query_inside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.render.query_inside_first(type_id, callback)
      && self.data.query_inside_first(type_id, callback)
  }

  fn query_outside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.data.query_outside_first(type_id, callback)
      && self.render.query_outside_first(type_id, callback)
  }
}

impl Query for AnonymousWrapper {
  crate::widget::impl_proxy_query!(render);
}

impl RenderTarget for AnonymousWrapper {
  type Target = dyn Render;
  #[inline]
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(&*self.render) }
}
