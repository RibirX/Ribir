use ribir::prelude::*;

fn main() {
  let _ = widget! {
    SizedBox {
      id: outside, size: Size::zero(),
      cursor: CursorIcon::Default,
      DynWidget {
        dyns: widget! {
          SizedBox {
            size: outside.size,
            cursor: outside.cursor.clone(),
          }
        }
      }
    }
  };
}
