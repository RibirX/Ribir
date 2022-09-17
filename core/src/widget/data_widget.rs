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
}

fn expr_attach_data<D: Query + Clone + 'static>(
  expr: ExprWidget<Box<dyn FnMut(&mut BuildCtx) -> DynamicWidget>>,
  children: Children,
  data: D,
) -> Widget {
  let ExprWidget { mut expr, upstream } = expr;
  let new_expr = move |ctx: &mut BuildCtx| match expr(ctx) {
    DynamicWidget::Single(w) => {
      let w = w.map(|w| widget_attach_data(w, data.clone(), expr_attach_data));
      DynamicWidget::Single(w)
    }
    DynamicWidget::Multi(m) => {
      let data = data.clone();
      let m = m.map(move |w| widget_attach_data(w, data.clone(), expr_attach_data));
      DynamicWidget::Multi(Box::new(m))
    }
  };

  let node = WidgetNode::Dynamic(ExprWidget { expr: Box::new(new_expr), upstream });
  Widget { node: Some(node), children }
}

pub fn compose_child_as_data_widget<D: Query + 'static>(
  child: Widget,
  data: StateWidget<D>,
) -> Widget {
  match data {
    StateWidget::Stateless(data) => widget_attach_data(
      child,
      data,
      |expr: ExprWidget<Box<dyn FnMut(&mut BuildCtx) -> DynamicWidget>>,
       children: Children,
       data: D| {
        let data = Stateful::new(data);
        expr_attach_data(expr, children, data)
      },
    ),
    StateWidget::Stateful(data) => widget_attach_data(child, data, expr_attach_data),
  }
}

fn widget_attach_data<D: Query + 'static>(
  widget: Widget,
  data: D,
  attach_expr: impl FnOnce(
    ExprWidget<Box<dyn FnMut(&mut BuildCtx) -> DynamicWidget>>,
    Children,
    D,
  ) -> Widget
  + 'static,
) -> Widget {
  let Widget { node, children } = widget;
  if let Some(node) = node {
    match node {
      WidgetNode::Compose(c) => {
        assert!(children.is_none());
        (|ctx: &mut BuildCtx| widget_attach_data(c(ctx), data, attach_expr)).into_widget()
      }
      WidgetNode::Render(r) => {
        let node = WidgetNode::Render(Box::new(DataWidget { widget: r, data }));
        Widget { node: Some(node), children }
      }
      WidgetNode::Dynamic(expr) => attach_expr(expr, children, data),
    }
  } else {
    match children {
      Children::None => Widget { node: None, children: Children::None },
      Children::Single(s) => widget_attach_data(*s, data, attach_expr),
      Children::Multi(_) => unreachable!("Compiler should not allow attach data to many widget."),
    }
  }
}
