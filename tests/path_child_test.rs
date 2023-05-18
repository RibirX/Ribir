use ribir::core::test_helper::*;
use ribir::prelude::*;
use ribir_dev_helper::*;
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
  let _ = widget! { AB::A };
  let _ = widget! { AB::B };
  let _ = widget! { AB::a() };
  let _ = widget! { AB::b() };
}

fn tuple_widget() -> Widget {
  struct TupleBox(Size);
  impl Compose for TupleBox {
    fn compose(this: State<Self>) -> Widget {
      widget! {
        states { this: this.into_readonly() }
        SizedBox { size: this.0 }
      }
    }
  }
  widget! { TupleBox(Size::new(1., 1.)) }
}
widget_layout_test!(tuple_widget, width == 1., height == 1.,);
