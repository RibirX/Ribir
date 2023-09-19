//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use crate::{impl_proxy_query, impl_proxy_render, prelude::*, widget::FnWidget};

pub struct DataWidget<D> {
  render: Box<dyn Render>,
  data: D,
}

impl<D: Query + 'static> DataWidget<D> {
  pub fn new(render: Box<dyn Render>, data: D) -> Self { DataWidget { render, data } }

  pub fn attach(widget: Widget, data: D) -> Widget {
    FnWidget::new(move |ctx| {
      let id = widget.build(ctx);
      id.wrap_node(&mut ctx.tree.borrow_mut().arena, |child| {
        Box::new(Self::new(child, data))
      });
      id
    })
    .into()
  }

  pub fn attach_state(widget: Widget, data: State<D>) -> Widget {
    match data.0.into_inner() {
      InnerState::Data(data) => {
        let data = data.into_inner();
        DataWidget::attach(widget, data)
      }
      InnerState::Stateful(data) => DataWidget::attach(widget, data),
    }
  }
}

impl_proxy_query!(paths [data, render], DataWidget<D>, <D>, where D: Query + 'static);
impl_proxy_render!(proxy render, DataWidget<D>, <D>, where D: Query + 'static);

/// A wrapper widget which can attach any data to a widget and not care about
/// what the data is.
pub struct AnonymousWrapper {
  widget: Box<dyn Render>,
  _data: Box<dyn Any>,
}

impl AnonymousWrapper {
  #[inline]
  pub fn new(widget: Box<dyn Render>, data: Box<dyn Any>) -> Self {
    AnonymousWrapper { widget, _data: data }
  }
}

impl Query for AnonymousWrapper {
  #[inline]
  fn query_inside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.widget.query_inside_first(type_id, callback)
  }
  #[inline]
  fn query_outside_first(&self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool) {
    self.widget.query_outside_first(type_id, callback)
  }
}

impl_proxy_render!(proxy widget, AnonymousWrapper);
