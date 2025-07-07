use ribir::{core::test_helper::*, prelude::*};
use ribir_dev_helper::*;
enum AB {
  A,
  B,
}

const SIZE_ONE: Size = Size::new(1., 1.);
impl Compose for AB {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @SizedBox {
        size: match *$read(this) {
          AB::A => ZERO_SIZE,
          AB::B => SIZE_ONE
        }
      }
    }
    .into_widget()
  }
}

impl AB {
  fn a() -> Self { AB::A }

  fn b() -> Self { AB::B }
}

#[test]
fn path_widget() {
  let _ = fn_widget! { AB::A };
  let _ = fn_widget! { AB::B };
  let _ = fn_widget! { AB::a() };
  let _ = fn_widget! { AB::b() };
}

struct TupleBox(Size);
impl Compose for TupleBox {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      @SizedBox {
        size: pipe!($read(this).0),
      }
    }
    .into_widget()
  }
}

widget_layout_test!(
  tuple_widget,
  WidgetTester::new(fn_widget! {
    TupleBox(Size::new(1., 1.))
  }),
  LayoutCase::default().with_size(Size::new(1., 1.))
);
