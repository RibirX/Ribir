use ribir::prelude::*;

fn main() {
  let _observe_on_outside_declare = widget! {
    Void {
      id: outside,
      DynWidget {
        dyns: widget!{
          Void {}
          on outside { tap: |_| { }}
        }
      }
    }
  };
}
