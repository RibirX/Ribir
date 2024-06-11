//! Data widget help attach data to a widget and get a new widget which behavior
//! is same as origin widget.

use widget_id::RenderQueryable;

use crate::{prelude::*, render_helper::RenderProxy};

pub(crate) struct DataAttacher<D: Query> {
  render: Box<dyn RenderQueryable>,
  data: D,
}

/// This is a wrapper for a data that makes it queryable.
pub struct Queryable<T: Any>(pub T);

/// A wrapper widget which can attach any data to a widget and not care about
/// what the data is.
pub(crate) struct AnonymousAttacher {
  render: Box<dyn RenderQueryable>,
  _data: Box<dyn Any>,
}

impl<D: Query> DataAttacher<D> {
  pub(crate) fn new(render: Box<dyn RenderQueryable>, data: D) -> Self {
    DataAttacher { render, data }
  }
}

impl AnonymousAttacher {
  #[inline]
  pub fn new(render: Box<dyn RenderQueryable>, data: Box<dyn Any>) -> Self {
    AnonymousAttacher { render, _data: data }
  }
}

// fixme: These APIs should be private, use Provide instead.
impl Widget {
  /// Attach data to a widget and user can query it.
  pub fn attach_data<D: Query>(self, data: D, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().attach_data(data, arena);

    self
  }

  /// Attach a state to a widget and try to unwrap it before attaching.
  ///
  /// User can query the state or its value type.
  pub fn try_unwrap_state_and_attach<D: Any>(
    self, data: impl StateWriter<Value = D>, ctx: &BuildCtx,
  ) -> Widget {
    match data.try_into_value() {
      Ok(data) => self.attach_data(Queryable(data), ctx),
      Err(data) => self.attach_data(data, ctx),
    }
  }

  /// Attach anonymous data to a widget and user can't query it.
  pub fn attach_anonymous_data(self, data: impl Any, ctx: &BuildCtx) -> Widget {
    let arena = &mut ctx.tree.borrow_mut().arena;
    self.id().attach_anonymous_data(data, arena);
    self
  }
}

impl<D: Query> RenderProxy for DataAttacher<D> {
  type R = dyn RenderQueryable;

  type Target<'r> = &'r dyn RenderQueryable
  where
    Self: 'r;

  fn proxy(&self) -> Self::Target<'_> { self.render.as_ref() }
}

impl<D: Query> Query for DataAttacher<D> {
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

impl<T: Any> Query for Queryable<T> {
  fn query_all(&self, type_id: TypeId) -> smallvec::SmallVec<[QueryHandle; 1]> {
    self.query(type_id).into_iter().collect()
  }

  fn query(&self, type_id: TypeId) -> Option<QueryHandle> {
    (type_id == self.0.type_id()).then(|| QueryHandle::new(&self.0))
  }
}
