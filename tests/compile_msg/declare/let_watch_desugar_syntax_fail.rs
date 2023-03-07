use ribir::prelude::*;

fn main() {
  let _ = widget!{
    SizedBox { id: sized_box, size: ZERO_SIZE }
    finally {
      let_watch!(sized_box.size).subscribe(|_| {})
    }
  };
}