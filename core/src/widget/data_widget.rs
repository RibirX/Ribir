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

fn widget_with_data<D: Query + 'static>(widget: Widget, data: D) -> Widget {
  match widget.0 {
    WidgetInner::Compose(c) => (|ctx: &mut BuildCtx| widget_with_data(c(ctx), data)).into_widget(),
    WidgetInner::Render(widget) => DataWidget { widget, data }.into_widget(),
    WidgetInner::SingleChild(s) => single_child_with_data(s, data),
    WidgetInner::MultiChild(m) => multi_child_with_data(m, data),
    WidgetInner::ExprWidget(_) => {
      let data = Stateful::new(data);
      widget_with_clone_data(widget, data)
    }
  }
}

fn widget_with_clone_data<D: Query + Clone + 'static>(widget: Widget, data: D) -> Widget {
  match widget.0 {
    WidgetInner::Compose(c) => {
      (|ctx: &mut BuildCtx| widget_with_clone_data(c(ctx), data)).into_widget()
    }
    WidgetInner::Render(widget) => DataWidget { widget, data }.into_widget(),
    WidgetInner::SingleChild(s) => single_child_with_data(s, data),
    WidgetInner::MultiChild(m) => multi_child_with_data(m, data),
    WidgetInner::ExprWidget(ExprWidget { mut expr, upstream }) => {
      let new_expr = move || match expr() {
        ExprResult::Single(w) => {
          let w = w.map(|w| widget_with_clone_data(w, data.clone()));
          ExprResult::Single(w)
        }
        ExprResult::Multi(mut v) => {
          v.iter_mut().for_each(|w| {
            let mut inner = std::mem::replace(w, Void.into_widget());
            inner = widget_with_clone_data(inner, data.clone());
            let _ = std::mem::replace(w, inner);
          });
          ExprResult::Multi(v)
        }
      };

      Widget(WidgetInner::ExprWidget(ExprWidget {
        expr: Box::new(new_expr),
        upstream,
      }))
    }
  }
}

pub fn compose_child_as_data_widget<D: Query + 'static>(
  child: Widget,
  data: StateWidget<D>,
) -> Widget {
  match data {
    StateWidget::Stateless(data) => widget_with_data(child, data),
    StateWidget::Stateful(data) => widget_with_clone_data(child, data),
  }
}

fn single_child_with_data<D: Query + 'static>(s: BoxedSingleChild, data: D) -> Widget {
  let widget: Box<dyn Render> = Box::new(DataWidget { widget: s.widget, data });
  let single = Box::new(SingleChildWidget { widget, child: s.child });
  Widget(WidgetInner::SingleChild(single))
}

fn multi_child_with_data<D: Query + 'static>(m: BoxedMultiChild, data: D) -> Widget {
  let widget: Box<dyn Render> = Box::new(DataWidget { widget: m.widget, data });
  let multi = MultiChildWidget { widget, children: m.children };
  Widget(WidgetInner::MultiChild(multi))
}
