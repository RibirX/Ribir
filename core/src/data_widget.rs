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
    self, data: impl StateReader<Value = D>, ctx: &BuildCtx,
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

  fn proxy<'r>(&'r self) -> Self::Target<'r> { self.render.as_ref() }
}

impl<D: Query> Query for DataAttacher<D> {
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

impl Query for AnonymousAttacher {
  fn query_inside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.render.query_inside_first(type_id, callback)
  }

  fn query_outside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.render.query_outside_first(type_id, callback)
  }
}

impl RenderProxy for AnonymousAttacher {
  type R = dyn RenderQueryable;

  type Target<'r> = &'r dyn RenderQueryable
  where
    Self: 'r;

  fn proxy<'r>(&'r self) -> Self::Target<'r> { self.render.as_ref() }
}

impl<T: Any> Query for Queryable<T> {
  #[inline]
  fn query_inside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    self.query_outside_first(type_id, callback)
  }

  #[inline]
  fn query_outside_first(
    &self, type_id: TypeId, callback: &mut dyn FnMut(&dyn Any) -> bool,
  ) -> bool {
    if type_id == TypeId::of::<T>() { callback(&self.0) } else { true }
  }
}
