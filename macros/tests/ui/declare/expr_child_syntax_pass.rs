use ribir::prelude::*;

fn main() {
  let size = Size::zero();
  let _child_always_declare_behind_field = widget! {
    SizedBox {
      size,
      DynWidget {
        dyns: if size.area() > 0. {
          SizedBox { size }
        } else {
          SizedBox { size }
        }
      }
    }
  };

  let _option_child = widget! {
    SizedBox {
      size,
      background: Color::RED,
      DynWidget {
        dyns: (size.area() == 0.).then(|| SizedBox { size } )
      }
    }
  };

  let _expr_child_use_named_widget = widget! {
    Flex {
      SizedBox { id: a, size }
      DynWidget {
        dyns: (a.size.area() > 0.).then(|| SizedBox { size })
      }
    }
  };
}
