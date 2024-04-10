use super::*;
use crate::pipe::InnerPipe;

/// Trait specify what child a widget can have, and the target type is the
/// result of widget compose its child.
pub trait SingleWithChild<C, M: ?Sized> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<P: SingleParent, C: {#}> SingleWithChild<C, dyn {#}> for P {
    type Target = Widget;
    #[track_caller]
    fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
      self.compose_child(child.build(ctx), ctx)
    }
  }

  impl<P, C> SingleWithChild<Option<C>, dyn {#}> for P
  where
    P: SingleParent + RenderBuilder,
    C: {#},
  {
    type Target = Widget;
    #[track_caller]
    fn with_child(self, child: Option<C>, ctx: &BuildCtx) -> Self::Target {
      if let Some(child) = child {
        self.with_child(child, ctx)
      } else {
        self.build(ctx)
      }
    }
  }
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<P, V, PP> SingleWithChild<PP, &dyn {#}> for P
  where
    P: SingleParent,
    PP: InnerPipe<Value=Option<V>>,
    V: {#} + 'static,
  {
    type Target = Widget;
    #[track_caller]
    fn with_child(self, child: PP, ctx: &BuildCtx) -> Self::Target {
      let child = crate::pipe::pipe_option_to_widget!(child, ctx);
      self.with_child(child, ctx)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::MockBox;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = |ctx| -> Widget {
      mock_box
        .clone()
        .with_child(mock_box.clone().with_child(mock_box, ctx), ctx)
        .build(ctx)
    };
  }

  #[test]
  fn fix_mock_box_compose_pipe_option_widget() {
    fn _x(w: BoxPipe<Option<Widget>>, ctx: &BuildCtx) {
      MockBox { size: ZERO_SIZE }.with_child(w.into_pipe(), ctx);
    }
  }
}
