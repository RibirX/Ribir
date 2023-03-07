use ribir::prelude::*;

fn main() {
  let size = Size::zero();
  let _use_id_declare_later = widget! {
    Flex {
      SizedBox {
        size,
        SizedBox{ id: grandson, size }
      }
      SizedBox { size: grandson.size }
    }
  };
}
