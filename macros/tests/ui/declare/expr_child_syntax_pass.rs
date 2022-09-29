use ribir::prelude::*;

fn main() {
  let size = Size::zero();
  let _child_always_declare_behind_field = widget! {
    SizedBox {
      size,
      ExprWidget {
        expr: if size.area() > 0. {
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
      ExprWidget {
        expr: (size.area() == 0.).then(|| SizedBox { size } )
      }
    }
  };

  let _expr_child_use_named_widget = widget! {
    Flex {
      SizedBox { id: a, size }
      ExprWidget {
        expr: (a.size.area() > 0.).then(|| SizedBox { size })
      }
    }
  };
}
