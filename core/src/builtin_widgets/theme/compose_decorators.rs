use std::any::type_name;

use crate::prelude::*;

type ComposeDecoratorFn = dyn Fn(Box<dyn Any>, Widget, &BuildCtx) -> Widget;
/// Compose style is a compose child widget to decoration its child.
#[derive(Default)]
pub struct ComposeDecorators {
  styles: ahash::HashMap<TypeId, Box<ComposeDecoratorFn>>,
}

/// `ComposeDecorator` is a trait let you can convert your host widget to
/// another, it has same signature of `ComposeChild`, but it can be overwrote in
/// `Theme` by a function. The trait implementation only as a default logic if
/// no overwrite function in `Theme`.
pub trait ComposeDecorator: Sized {
  fn compose_decorator(this: State<Self>, host: Widget) -> impl WidgetBuilder;
}

// todo: remove it, keep it for backward compatibility.
// `ComposeDecorator` without share state should not implement as a
// `ComposeDecorator`.
impl<M, T, C> ComposeWithChild<C, [M; 100]> for T
where
  T: ComposeDecorator,
  State<T>: ComposeWithChild<C, M>,
{
  type Target = <State<T> as ComposeWithChild<C, M>>::Target;
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    State::value(self).with_child(child, ctx)
  }
}

impl<M, W, C> ComposeWithChild<C, [M; 101]> for State<W>
where
  W: ComposeDecorator + 'static,
  Widget: ChildFrom<C, M>,
{
  type Target = Widget;
  #[track_caller]
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target {
    let tid = TypeId::of::<W>();
    let style = ctx.find_cfg(|t| match t {
      Theme::Full(t) => t.compose_decorators.styles.get(&tid),
      Theme::Inherit(i) => i
        .compose_decorators
        .as_ref()
        .and_then(|s| s.styles.get(&tid)),
    });

    let host = ChildFrom::child_from(child, ctx);
    if let Some(style) = style {
      style(Box::new(self), host, ctx)
    } else {
      ComposeDecorator::compose_decorator(self, host).build(ctx)
    }
  }
}

impl ComposeDecorators {
  #[inline]
  pub fn override_compose_decorator<W: ComposeDecorator + 'static>(
    &mut self, compose_decorator: impl Fn(State<W>, Widget, &BuildCtx) -> Widget + 'static,
  ) {
    self.styles.insert(
      TypeId::of::<W>(),
      Box::new(move |this: Box<dyn Any>, host: Widget, ctx: &BuildCtx| {
        let this = this.downcast().unwrap_or_else(|_| {
          panic!("Caller should guarantee the boxed type is Stateful<{}>.", type_name::<W>())
        });

        compose_decorator(*this, host, ctx)
      }),
    );
  }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use crate::{prelude::*, reset_test_env, test_helper::*};

  #[test]
  fn compose_decorator_smoke() {
    reset_test_env!();

    let mut theme = FullTheme::default();

    #[derive(Declare)]
    struct Size100Style;

    impl ComposeDecorator for Size100Style {
      fn compose_decorator(_: State<Self>, host: Widget) -> impl WidgetBuilder { fn_widget!(host) }
    }
    theme
      .compose_decorators
      .override_compose_decorator::<Size100Style>(|_, host, ctx| {
        fn_widget! {
          @MockBox {
            size: Size::new(100., 100.),
            @ { host }
          }
        }
        .build(ctx)
      });

    let w = fn_widget! {
      @Size100Style { @MockBox {
        size: Size::zero(),
      }}
    };

    unsafe { AppCtx::set_app_theme(theme) };

    let mut wnd = TestWindow::new_with_size(w, Size::new(500., 500.));
    wnd.draw_frame();
    assert_layout_result_by_path!(
      wnd,
      { path = [0], width == 100., height == 100., }
    );
  }
}
