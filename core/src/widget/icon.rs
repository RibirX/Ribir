use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Icon {
  pub size: Size,
}

impl ComposeSingleChild for Icon {
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    // todo:
    // Maybe we can not subscribe the stateful `this` in codegen if nobody want to
    // modify `this`, across compare its ref count and the count of who just
    // follow its change .  So user can directly track but codegen will not
    // subscribe if no triggers have.
    widget! {
      track { this }
      SizedBox {
        size: this.size,
        ExprWidget { expr: child }
      }
    }
  }
}
