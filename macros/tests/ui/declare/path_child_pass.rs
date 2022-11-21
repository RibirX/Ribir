use ribir::core::test::*;
use ribir::prelude::*;
enum AB {
  A,
  B,
}

const SIZE_ONE: Size = Size::new(1., 1.);
impl Compose for AB {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      track { this: this.into_stateful() }
      SizedBox {
        size: match *this {
          AB::A => ZERO_SIZE,
          AB::B => SIZE_ONE
        }
      }
    }
  }
}

impl AB {
  fn a() -> Self { AB::A }

  fn b() -> Self { AB::B }
}

fn main() {
  expect_layout_result(
    widget! { AB::A },
    None,
    &[LayoutTestItem {
      path: &[0],
      expect: ExpectRect::from_size(ZERO_SIZE),
    }],
  );

  expect_layout_result(
    widget! { AB::B },
    None,
    &[LayoutTestItem {
      path: &[0],
      expect: ExpectRect::from_size(SIZE_ONE),
    }],
  );

  expect_layout_result(
    widget! { AB::a() },
    None,
    &[LayoutTestItem {
      path: &[0],
      expect: ExpectRect::from_size(ZERO_SIZE),
    }],
  );

  expect_layout_result(
    widget! { AB::b() },
    None,
    &[LayoutTestItem {
      path: &[0],
      expect: ExpectRect::from_size(SIZE_ONE),
    }],
  );
}
