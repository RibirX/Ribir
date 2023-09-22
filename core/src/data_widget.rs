//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use crate::{impl_proxy_query, impl_proxy_render, prelude::*, render_helper::RenderTarget};

pub struct DataWidget<D> {
  render: Box<dyn Render>,
  data: D,
}

impl_proxy_query!(paths [data, render], DataWidget<D>, <D>, where D: Query + 'static);
impl_proxy_render!(proxy render, DataWidget<D>, <D>, where D: Query + 'static);

/// A wrapper widget which can attach any data to a widget and not care about
/// what the data is.
pub struct AnonymousWrapper {
  render: Box<dyn Render>,
  _data: Box<dyn Any>,
}

impl<D> DataWidget<D> {
  pub(crate) fn new(render: Box<dyn Render>, data: D) -> Self { DataWidget { render, data } }
}

impl AnonymousWrapper {
  #[inline]
  pub fn new(render: Box<dyn Render>, data: Box<dyn Any>) -> Self {
    AnonymousWrapper { render, _data: data }
  }
}

impl RenderTarget for AnonymousWrapper {
  type Target = dyn Render;
  #[inline]
  fn proxy<V>(&self, f: impl FnOnce(&Self::Target) -> V) -> V { f(&*self.render) }
}

impl Widget {
  pub fn attach_data<D: Query>(self, data: D, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().attach_data(data, arena);

    self
  }

  pub fn attach_state_data<D: Query>(self, data: State<D>, ctx: &BuildCtx) -> Widget {
    match data.0.into_inner() {
      InnerState::Data(data) => {
        let data = data.into_inner();
        self.attach_data(data, ctx)
      }
      InnerState::Stateful(data) => self.attach_data(data, ctx),
    }
  }

  pub fn attach_anonymous_data(self, data: impl Any, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().attach_anonymous_data(data, arena);
    self
  }
}
