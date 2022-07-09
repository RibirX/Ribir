use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Icon {
  pub size: Size,
}

impl ComposeSingleChild for Icon {
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    widget! {
      track { this }
      SizedBox {
        size: this.size,
        ExprWidget {
          expr: child,
          box_fit: BoxFit::Contain,
          h_align: HAlign::Center,
          v_align: VAlign::Center,
        }
      }
    }
  }
}
