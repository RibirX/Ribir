//! Data widget help attach data to a widget and get a new widget witch behavior
//! is same as origin widget.

use crate::{impl_proxy_query, impl_proxy_render, prelude::*};

pub struct DataWidget<W, D> {
  widget: W,
  data: D,
}

impl<W, D> DataWidget<W, D> {
  #[inline]
  pub fn new(widget: W, data: D) -> Self { Self { widget, data } }
}

impl<W: Render + 'static, D: Query + 'static> Render for DataWidget<W, D> {
  impl_proxy_render!(widget);
}

impl<W: Query + 'static, D: Query + 'static> Query for DataWidget<W, D> {
  impl_proxy_query!(self.data, self.widget);
}

pub fn compose_child_as_data_widget<D: Query + 'static>(
  child: Widget,
  data: StateWidget<D>,
) -> Widget {
  match data {
    StateWidget::Stateless(data) => widget_attach_data(child, data),
    StateWidget::Stateful(data) => widget_attach_data(child, data),
  }
}

pub fn widget_attach_data<D: Query + 'static>(widget: Widget, data: D) -> Widget {
  let Widget { node, mut children } = widget;
  if let Some(node) = node {
    match node {
      WidgetNode::Compose(c) => {
        assert!(children.is_empty());
        (|ctx: &BuildCtx| widget_attach_data(c(ctx), data)).into_widget()
      }
      WidgetNode::Render(r) => {
        let node = WidgetNode::Render(Box::new(DataWidget { widget: r, data }));
        Widget { node: Some(node), children }
      }
    }
  } else {
    match children.len() {
      0 => Widget { node: None, children },
      1 => widget_attach_data(children.pop().unwrap(), data),
      _ => unreachable!("Compiler should not allow attach data to many widget."),
    }
  }
}
