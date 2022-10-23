use ribir_core::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};

/// A widget that expanded a child of `Flex`, so that the child fills the
/// available space. If multiple children are expanded, the available space is
/// divided among them according to the flex factor.
#[derive(Clone, PartialEq, Declare)]
pub struct Expanded {
  pub flex: f32,
}

impl ComposeChild for Expanded {
  type Child = Widget;
  #[inline]
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    compose_child_as_data_widget(child, this)
  }
}

impl Query for Expanded {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test::widget_and_its_children_box_rect;

  #[test]
  fn one_line_expanded() {
    let widget = |size| {
      widget! {
        Row {
          Expanded {
            flex: 1.,
            SizedBox { size }
          }
          SizedBox { size }
          SizedBox { size }
          Expanded {
            flex: 2.,
            SizedBox { size }
          }
        }
      }
    };

    let size = Size::new(100., 50.);

    let (rect, children) = widget_and_its_children_box_rect(widget(size), Size::new(500., 500.));

    assert_eq!(rect, Rect::from_size(Size::new(500., 50.)));
    assert_eq!(
      children,
      vec![
        Rect::from_size(size),
        Rect::new(Point::new(100., 0.), size),
        Rect::new(Point::new(200., 0.), size),
        Rect::new(Point::new(300., 0.), Size::new(200., 50.))
      ]
    )
  }

  #[test]
  fn wrap_expanded() {
    let size = Size::new(100., 50.);
    let row = widget! {
      Row {
        wrap: true,
        Expanded {
          flex: 1. ,
          SizedBox { size }
        }
        SizedBox { size }
        SizedBox { size }
        Expanded {
          flex: 2.,
          SizedBox { size }
        }
      }
    };

    let (rect, children) =
      widget_and_its_children_box_rect(row.into_widget(), Size::new(350., 500.));

    assert_eq!(rect, Rect::from_size(Size::new(350., 100.)));
    assert_eq!(
      children,
      vec![
        Rect::from_size(Size::new(150., 50.)),
        Rect::new(Point::new(150., 0.), size),
        Rect::new(Point::new(250., 0.), size),
        Rect::new(Point::new(0., 50.), Size::new(350., 50.))
      ]
    )
  }
}
