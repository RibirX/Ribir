use ribir::prelude::*;

fn main() {
  let _ = widget! {
    SizedBox {
      id: outside, size: Size::zero(),
      cursor: CursorIcon::Default,
      ExprWidget {
        expr: widget! {
          SizedBox {
            size: outside.size,
            cursor: outside.cursor.clone(),
          }
        }
      }
    }
  };
}
