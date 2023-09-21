use super::*;

/// Trait specify what child a widget can have, and the target type is the
/// result of widget compose its child.
pub trait SingleWithChild<C, M: ?Sized> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

crate::widget::multi_build_replace_impl_include_self! {
  impl<P: SingleParent, C: {#}> SingleWithChild<C, dyn {#}> for P {
    type Target = Widget;

    fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
      self.compose_child(child.widget_build(ctx), ctx)
    }
  }

  impl<P, C> SingleWithChild<Option<C>, dyn {#}> for P
  where
    P: SingleParent + {#},
    C: {#},
  {
    type Target = Widget;

    fn with_child(self, child: Option<C>, ctx: &BuildCtx) -> Self::Target {
      if let Some(child) = child {
        self.with_child(child, ctx)
      } else {
        self.widget_build(ctx)
      }
    }
  }

  impl<P, C: 'static> SingleWithChild<Pipe<Option<C>>, dyn {#}> for P
  where
    P: SingleParent + {#},
    C: {#},
  {
    type Target = Widget;

    fn with_child(self, child: Pipe<Option<C>>, ctx: &BuildCtx) -> Self::Target {
      let child = crate::pipe::pipe_option_to_widget!(child, ctx);
      self.with_child(child, ctx)
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test_helper::MockBox;

  use super::*;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = |ctx| -> Widget {
      mock_box
        .clone()
        .with_child(mock_box.clone().with_child(mock_box, ctx), ctx)
        .widget_build(ctx)
    };
  }
}
