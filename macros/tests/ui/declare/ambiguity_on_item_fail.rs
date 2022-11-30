use ribir::prelude::*;

fn main() {
  let miss_block = widget! {
    Container {
      id: container,
      size: Size::zero() }
    on container.size
  };

  let ambiguity = widget! {
    states { flag: true.into_stateful() }
    Container {
      size: Size::zero()
    }
    on *flag {
      change: |_, _| {}
    }
  };
}
