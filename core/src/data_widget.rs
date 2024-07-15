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

// fixme: These APIs should be removed, use Provide instead.
impl<'a> Widget<'a> {
  /// Attach data to a widget and user can query it.
  pub fn attach_data<D: Query>(self, data: D) -> Widget<'a> {
    let f = move |ctx: &BuildCtx| {
      let w = self.build(ctx);
      w.attach_data(data, &mut ctx.tree.borrow_mut());
      w
    };
    InnerWidget::LazyBuild(Box::new(f)).into()
  }

  /// Attach a state to a widget and try to unwrap it before attaching.
  ///
  /// User can query the state or its value type.
  pub fn try_unwrap_state_and_attach<D: Any>(
    self, data: impl StateWriter<Value = D> + 'static,
  ) -> Widget<'a> {
    match data.try_into_value() {
      Ok(data) => self.attach_data(Queryable(data)),
      Err(data) => self.attach_data(data),
    }
  }

  /// Attach anonymous data to a widget and user can't query it.
  pub fn attach_anonymous_data(self, data: impl Any) -> Widget<'a> {
    let f = move |ctx: &BuildCtx| {
      let w = self.build(ctx);
      w.attach_anonymous_data(data, &mut ctx.tree.borrow_mut());
      w
    };

    InnerWidget::LazyBuild(Box::new(f)).into()
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
