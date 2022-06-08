//! Data widget help attach data to a widget and get a new widget witch behavior
//! is same as origin widget.

use crate::prelude::*;
use std::rc::Rc;

pub struct DataWidget<W, D> {
  widget: W,
  data: D,
}

impl<W, D> DataWidget<W, Stateful<D>> {
  #[inline]
  pub fn new(widget: W, data: Stateful<D>) -> Self { Self { widget, data } }
}

impl<D: Query> Render for DataWidget<Box<dyn Render>, D> {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    self.widget.perform_layout(clamp, ctx)
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) { self.widget.paint(ctx) }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { self.widget.only_sized_by_parent() }
}

impl<D: Query> Query for DataWidget<Box<dyn Render>, D> {
  fn query_all(
    &self,
    type_id: std::any::TypeId,
    callback: &mut dyn FnMut(&dyn Any) -> bool,
    order: QueryOrder,
  ) {
    let mut continue_query = true;
    match order {
      QueryOrder::InnerFirst => {
        self.widget.query_all(
          type_id,
          &mut |t| {
            continue_query = callback(t);
            continue_query
          },
          order,
        );
        if continue_query {
          self.data.query_all(type_id, callback, order);
        }
      }
      QueryOrder::OutsideFirst => {
        self.data.query_all(
          type_id,
          &mut |t| {
            continue_query = callback(t);
            continue_query
          },
          order,
        );
        if continue_query {
          self.widget.query_all(type_id, callback, order);
        }
      }
    }
  }

  fn query_all_mut(
    &mut self,
    type_id: std::any::TypeId,
    callback: &mut dyn FnMut(&mut dyn Any) -> bool,
    order: QueryOrder,
  ) {
    let mut continue_query = true;
    match order {
      QueryOrder::InnerFirst => {
        self.widget.query_all_mut(
          type_id,
          &mut |t| {
            continue_query = callback(t);
            continue_query
          },
          order,
        );
        if continue_query {
          self.data.query_all_mut(type_id, callback, order);
        }
      }
      QueryOrder::OutsideFirst => {
        self.data.query_all_mut(
          type_id,
          &mut |t| {
            continue_query = callback(t);
            continue_query
          },
          order,
        );
        if continue_query {
          self.widget.query_all_mut(type_id, callback, order);
        }
      }
    }
  }
}

impl<W: Query + 'static> DataWidget<Widget, Stateful<W>> {
  pub fn into_widget_and_try_unwrap_data<D: Query + 'static>(
    self,
    pick_data: impl FnOnce(W) -> D + Clone + 'static,
  ) -> Widget {
    let data = self.data;
    match self.widget.0 {
      WidgetInner::Compose(c) => {
        |ctx: &mut BuildCtx| {
          DataWidget { widget: c(ctx), data }.into_widget_and_try_unwrap_data(pick_data)
        }
      }
      .into_widget(),
      WidgetInner::Render(widget) => {
        let r = DataWidget { widget, data }.into_render_node(pick_data);
        Widget(WidgetInner::Render(r))
      }
      WidgetInner::SingleChild(s) => {
        let widget: Box<dyn Render> =
          DataWidget { widget: s.widget, data }.into_render_node(pick_data);
        let single = Box::new(SingleChildWidget { widget, child: s.child });
        Widget(WidgetInner::SingleChild(single))
      }
      WidgetInner::MultiChild(m) => {
        let widget: Box<dyn Render> =
          DataWidget { widget: m.widget, data }.into_render_node(pick_data);
        let multi = MultiChildWidget { widget, children: m.children };
        Widget(WidgetInner::MultiChild(multi))
      }
      WidgetInner::Expr(ExprWidget { mut expr, upstream }) => {
        let new_expr = move |cb: &mut dyn FnMut(Widget)| {
          expr(&mut |widget| {
            let w = DataWidget { widget, data: data.clone() }
              .into_widget_and_try_unwrap_data(pick_data.clone());
            cb(w)
          })
        };
        Widget(WidgetInner::Expr(ExprWidget {
          expr: Box::new(new_expr),
          upstream,
        }))
      }
    }
  }
}

impl<W: Query + 'static> DataWidget<Box<dyn Render>, Stateful<W>> {
  fn into_render_node<D: 'static + Query>(self, pick_data: impl FnOnce(W) -> D) -> Box<dyn Render> {
    let Self { widget, data } = self;
    match Rc::try_unwrap(data.widget) {
      Ok(d) => Box::new(DataWidget {
        widget,
        data: pick_data(d.into_inner()),
      }),
      Err(d) => Box::new(DataWidget {
        widget,
        data: Stateful {
          widget: d,
          change_notifier: data.change_notifier,
        },
      }),
    }
  }
}
