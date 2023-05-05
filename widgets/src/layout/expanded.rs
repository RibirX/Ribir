use ribir_core::{data_widget::compose_child_as_data_widget, impl_query_self_only, prelude::*};

use super::ConstrainedBox;

/// A widget that expanded a child of `Flex`, so that the child fills the
/// available space. If multiple children are expanded, the available space is
/// divided among them according to the flex factor.
#[derive(Clone, PartialEq, Declare)]
pub struct Expanded {
  pub flex: f32,
}

impl ComposeChild for Expanded {
  type Child = Widget;
  type Target = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Self::Target {
    let w = widget! {
      ConstrainedBox {
        clamp: BoxClamp {
          min: Size::new(0., 0.),
          max: Size::new(f32::INFINITY, f32::INFINITY)
        },
        DynWidget {
          dyns: child
        }
      }
    };
    compose_child_as_data_widget(w.into_widget(), this)
  }
}

impl Query for Expanded {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test::*;

  #[test]
  fn expaned_child_size_zero() {
    let size = Size::new(100., 50.);

    let widget = widget! {
      Row {
        Expanded {
          flex: 1.,
          SizedBox { size }
        }
        SizedBox { size }
        Expanded {
          flex: 2.,
          SizedBox { size: Size::new(0., 50.) }
        }
      }
    };

    expect_layout_result(
      widget,
      Some(Size::new(500., 500.)),
      &[
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::from_size(Size::new(400., 50.)),
        },
        LayoutTestItem {
          path: &[0, 2],
          expect: ExpectRect::from_size(Size::new(0., 50.)),
        },
      ],
    );
  }

  #[test]
  fn one_line_expanded() {
    let size = Size::new(100., 50.);
    let widget = widget! {
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
    };

    expect_layout_result(
      widget,
      Some(Size::new(500., 500.)),
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::from_size(Size::new(500., 50.)),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::from_size(size),
        },
        LayoutTestItem {
          path: &[0, 1],
          expect: ExpectRect::new(100., 0., size.width, size.height),
        },
        LayoutTestItem {
          path: &[0, 2],
          expect: ExpectRect::new(200., 0., size.width, size.height),
        },
        LayoutTestItem {
          path: &[0, 3],
          expect: ExpectRect::new(300., 0., 200., 50.),
        },
      ],
    );
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
        SizedBox { size }
        SizedBox { size }
        Expanded {
          flex: 1. ,
          SizedBox { size, }
        }
        Expanded {
          flex: 4.,
          SizedBox { size, }
        }
      }
    };

    expect_layout_result(
      row,
      Some(Size::new(350., 500.)),
      &[
        LayoutTestItem {
          path: &[0],
          expect: ExpectRect::new(0., 0., 350., 100.),
        },
        LayoutTestItem {
          path: &[0, 0],
          expect: ExpectRect::new(0., 0., 50., 50.),
        },
        LayoutTestItem {
          path: &[0, 1],
          expect: ExpectRect::new(50., 0., size.width, size.height),
        },
        LayoutTestItem {
          path: &[0, 2],
          expect: ExpectRect::new(150., 0., size.width, size.height),
        },
        LayoutTestItem {
          path: &[0, 3],
          expect: ExpectRect::new(250., 0., size.width, size.height),
        },
        LayoutTestItem {
          path: &[0, 4],
          expect: ExpectRect::new(0., 50., size.width, size.height),
        },
        LayoutTestItem {
          path: &[0, 5],
          expect: ExpectRect::new(100., 50., 50., 50.),
        },
        LayoutTestItem {
          path: &[0, 6],
          expect: ExpectRect::new(150., 50., 200., 50.),
        },
      ],
    );
  }
}
