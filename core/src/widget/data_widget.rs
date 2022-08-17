//! Data widget help attach data to a widget and get a new widget witch behavior
//! is same as origin widget.

use crate::prelude::*;

pub struct DataWidget<W, D> {
  widget: W,
  data: D,
}

impl<W, D> DataWidget<W, D> {
  #[inline]
  pub fn new(widget: W, data: D) -> Self { Self { widget, data } }
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

fn widget_with_data<D: Query + 'static>(
  widget: Widget,
  data: D,
  expr_wrap: impl FnOnce(
    Box<dyn FnMut(&mut dyn FnMut(Widget))>,
    D,
  ) -> Box<dyn FnMut(&mut dyn FnMut(Widget))>
  + 'static,
) -> Widget {
  match widget.0 {
    WidgetInner::Compose(c) => {
      (|ctx: &mut BuildCtx| widget_with_data(c(ctx), data, expr_wrap)).into_widget()
    }
    WidgetInner::Render(widget) => {
      let r = Box::new(DataWidget { widget, data });
      Widget(WidgetInner::Render(r))
    }
    WidgetInner::SingleChild(s) => {
      let widget: Box<dyn Render> = Box::new(DataWidget { widget: s.widget, data });
      let single = Box::new(SingleChildWidget { widget, child: s.child });
      Widget(WidgetInner::SingleChild(single))
    }
    WidgetInner::MultiChild(m) => {
      let widget: Box<dyn Render> = Box::new(DataWidget { widget: m.widget, data });
      let multi = MultiChildWidget { widget, children: m.children };
      Widget(WidgetInner::MultiChild(multi))
    }
    WidgetInner::Expr(ExprWidget { expr, upstream }) => {
      let new_expr = expr_wrap(expr, data);

      Widget(WidgetInner::Expr(ExprWidget { expr: new_expr, upstream }))
    }
  }
}

pub fn compose_child_as_data_widget<D: Query + 'static>(
  child: Option<Widget>,
  data: StateWidget<D>,
) -> Widget {
  if let Some(child) = child {
    match data {
      StateWidget::Stateless(data) => widget_with_data(child, data, data_wrap),
      StateWidget::Stateful(data) => widget_with_data(child, data, clone_data_wrap),
    }
  } else {
    Void.into_widget()
  }
}
fn data_wrap<D: Query + 'static>(
  expr: Box<dyn FnMut(&mut dyn FnMut(Widget))>,
  data: D,
) -> Box<dyn FnMut(&mut dyn FnMut(Widget))> {
  let data = Stateful::new(data);
  clone_data_wrap(expr, data)
}

fn clone_data_wrap<D: Query + Clone + 'static>(
  mut expr: Box<dyn FnMut(&mut dyn FnMut(Widget))>,
  data: D,
) -> Box<dyn FnMut(&mut dyn FnMut(Widget))> {
  let new_expr = move |cb: &mut dyn FnMut(Widget)| {
    expr(&mut |widget| {
      let w = widget_with_data(widget, data.clone(), clone_data_wrap);
      cb(w)
    })
  };
  Box::new(new_expr)
}
