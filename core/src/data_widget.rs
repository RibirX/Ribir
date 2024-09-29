//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use smallvec::SmallVec;
use widget_id::RenderQueryable;

use crate::{prelude::*, render_helper::RenderProxy};

pub(crate) struct DataAttacher {
  render: Box<dyn RenderQueryable>,
  data: Box<dyn Query>,
}

/// A wrapper widget which can attach any data to a widget and not care about
/// what the data is.
pub(crate) struct AnonymousAttacher {
  render: Box<dyn RenderQueryable>,
  _data: Box<dyn Any>,
}

impl DataAttacher {
  pub(crate) fn new(render: Box<dyn RenderQueryable>, data: Box<dyn Query>) -> Self {
    DataAttacher { render, data }
  }
}

impl AnonymousAttacher {
  #[inline]
  pub fn new(render: Box<dyn RenderQueryable>, data: Box<dyn Any>) -> Self {
    AnonymousAttacher { render, _data: data }
  }
}

impl RenderProxy for DataAttacher {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.render.as_ref() }
}

impl Query for DataAttacher {
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all(type_id, out);
    if let Some(h) = self.data.query(type_id) {
      out.push(h)
    }
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    self
      .render
      .query(type_id)
      .or_else(|| self.data.query(type_id))
  }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> {
    self
      .render
      .query_write(type_id)
      .or_else(|| self.data.query_write(type_id))
  }

  fn queryable(&self) -> bool { true }
}

impl Query for AnonymousAttacher {
  #[inline]
  fn query_all<'q>(&'q self, type_id: TypeId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all(type_id, out)
  }

  #[inline]
  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.render.query(type_id) }

  fn query_write(&self, type_id: TypeId) -> Option<QueryHandle> { self.render.query_write(type_id) }

  fn queryable(&self) -> bool { self.render.queryable() }
}

impl RenderProxy for AnonymousAttacher {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.render.as_ref() }
}
