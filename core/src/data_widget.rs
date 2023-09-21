//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use crate::{impl_proxy_query, impl_proxy_render, prelude::*};

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

impl AnonymousWrapper {
  #[inline]
  pub fn new(render: Box<dyn Render>, data: Box<dyn Any>) -> Self {
    AnonymousWrapper { render, _data: data }
  }
}

impl Query for AnonymousWrapper {
  #[inline]
  fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.render.query_inside_first(type_id, callback)
  }
  #[inline]
  fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.render.query_outside_first(type_id, callback)
  }
}

impl_proxy_render!(proxy render, AnonymousWrapper);

impl Widget {
  pub fn attach_data<D: Query>(self, data: D, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self
      .id()
      .wrap_node(arena, |render| Box::new(DataWidget { render, data }));

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

  pub fn attach_anonymous_data(self, data: Box<dyn Any>, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().wrap_node(arena, |render| {
      Box::new(AnonymousWrapper::new(render, data))
    });

    self
  }
}
