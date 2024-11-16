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
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all(query_id, out);
    if let Some(h) = self.data.query(query_id) {
      out.push(h)
    }
  }

  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self
      .render
      .query(query_id)
      .or_else(|| self.data.query(query_id))
  }

  fn query_write(&self, query_id: &QueryId) -> Option<QueryHandle> {
    self
      .render
      .query_write(query_id)
      .or_else(|| self.data.query_write(query_id))
  }

  fn queryable(&self) -> bool { true }

  fn query_match(
    &self, ids: &[QueryId], filter: &dyn Fn(&QueryId, &QueryHandle) -> bool,
  ) -> Option<(QueryId, QueryHandle)> {
    self
      .render
      .query_match(ids, filter)
      .or_else(|| self.data.query_match(ids, filter))
  }
}

impl Query for AnonymousAttacher {
  #[inline]
  fn query_all<'q>(&'q self, query_id: &QueryId, out: &mut SmallVec<[QueryHandle<'q>; 1]>) {
    self.render.query_all(query_id, out)
  }

  #[inline]
  fn query(&self, query_id: &QueryId) -> Option<QueryHandle> { self.render.query(query_id) }

  fn query_write(&self, type_id: &QueryId) -> Option<QueryHandle> {
    self.render.query_write(type_id)
  }

  fn queryable(&self) -> bool { self.render.queryable() }

  fn query_match(
    &self, ids: &[QueryId], filter: &dyn Fn(&QueryId, &QueryHandle) -> bool,
  ) -> Option<(QueryId, QueryHandle)> {
    self.render.query_match(ids, filter)
  }
}

impl RenderProxy for AnonymousAttacher {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { self.render.as_ref() }
}
