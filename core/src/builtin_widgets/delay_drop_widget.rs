use crate::{impl_query_self_only, prelude::*};

#[derive(Declare, Declare2)]
pub struct DelayDropWidget {
  #[declare(builtin)]
  pub delay_drop_until: bool,
}

impl ComposeChild for DelayDropWidget {
  type Child = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    DataWidget::attach(child, this.into_writable())
  }
}

impl_query_self_only!(DelayDropWidget);
