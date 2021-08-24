use ribir::prelude::*;

fn child_always_declare_behind_field() {
  let size = Size::zero();
  let _ = declare! {
    SizedBox {
      size,
      if size.area() > 0. {
        SizedBox { size }.box_it()
      } else {
        declare!{
          SizedBox { size }
        }
      }

    }
  };
}

fn option_child() {
  let size = Size::zero();
  let _ = declare! {
    SizedBox {
      size,
      background: Color::RED,
      (size.area() == 0.).then(||{
        SizedBox { size }
      })
    }
  };
}

fn expr_child_use_named_widget() {
  let size = Size::zero();
  let _ = declare! {
    Flex {
      ..<_>::default(),
      SizedBox {
        id: a,
        size,
      }
      (a.size.area() > 0.).then(||
        SizedBox {
          size,
        }
      )
    }
  };
}

fn main() {}
