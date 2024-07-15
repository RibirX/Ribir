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
        size: match *$this {
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

fn tuple_widget() -> Widget<'static> {
  struct TupleBox(Size);
  impl Compose for TupleBox {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
      fn_widget! {
        @SizedBox {
          size: pipe!($this.0),
        }
      }
      .into_widget()
    }
  }
  TupleBox(Size::new(1., 1.)).into_widget()
}
widget_layout_test!(tuple_widget, width == 1., height == 1.,);
