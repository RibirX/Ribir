//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

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
  type R = dyn RenderQueryable;

  type Target<'r> = &'r dyn RenderQueryable
  where
    Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.render.as_ref() }
}

impl Query for DataAttacher {
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    let mut types = self.render.query_all(type_id);
    types.extend(self.data.query_all(type_id));
    types
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    self
      .data
      .query(type_id)
      .or_else(|| self.render.query(type_id))
  }
}

impl Query for AnonymousAttacher {
  #[inline]
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    self.render.query_all(type_id)
  }

  #[inline]
  fn query(&self, type_id: TypeId) -> Option<QueryHandle> { self.render.query(type_id) }
}

impl RenderProxy for AnonymousAttacher {
  type R = dyn RenderQueryable;

  type Target<'r> = &'r dyn RenderQueryable
  where
    Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.render.as_ref() }
}
