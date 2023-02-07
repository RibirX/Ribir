use ribir::prelude::*;

fn main() {
  let _flow_simple = widget! {
    Flex {
      SizedBox {
        id: a,
        size: Size::zero(),
      }
      SizedBox {
        size:= assign_watch!(a.size).stream_map(|o| o.distinct_until_changed()),
      }
    }
  };
}
