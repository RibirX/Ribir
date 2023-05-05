use crate::{impl_query_self_only, prelude::*, widget::TreeArena};

#[derive(Declare)]
pub struct DelayDropWidget {
  #[declare(builtin)]
  pub delay_drop_until: bool,
}

impl ComposeChild for DelayDropWidget {
  type Child = Widget;
  type Target = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Self::Target {
    let this = match this {
      State::Stateless(w) => Stateful::new(w),
      State::Stateful(w) => w,
    };
    widget_attach_data(child, this)
  }
}

impl Query for DelayDropWidget {
  impl_query_self_only!();
}

pub(crate) fn query_drop_until_widget(
  wid: WidgetId,
  arena: &TreeArena,
) -> Option<Stateful<DelayDropWidget>> {
  let mut drop_widget = None;
  wid
    .assert_get(arena)
    .query_on_first_type(QueryOrder::OutsideFirst, |w: &Stateful<DelayDropWidget>| {
      drop_widget = Some(w.clone_stateful())
    });
  drop_widget
}
