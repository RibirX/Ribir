use ribir::core::test::*;
use ribir::prelude::*;
enum AB {
  A,
  B,
}

const SIZE_ONE: Size = Size::new(1., 1.);
impl Compose for AB {
  fn compose(this: State<Self>) -> Widget {
    widget! {
      states { this: this.into_writable() }
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

#[test]
fn path_widget() {
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

#[test]
fn tuple_widget() {
  struct TupleBox(Size);
  impl Compose for TupleBox {
    fn compose(this: State<Self>) -> Widget {
      widget! {
        states { this: this.into_readonly() }
        SizedBox { size: this.0 }
      }
    }
  }

  let size = Size::new(1., 1.);
  expect_layout_result(
    widget! { TupleBox(size) },
    None,
    &[LayoutTestItem {
      path: &[0],
      expect: ExpectRect::from_size(size),
    }],
  );
}
