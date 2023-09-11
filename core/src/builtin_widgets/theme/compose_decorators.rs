use crate::{prelude::*, widget::FnWidget};
use std::{any::type_name, collections::HashMap};

type ComposeDecoratorFn = dyn Fn(Box<dyn Any>, Box<dyn Any>) -> Widget;
/// Compose style is a compose child widget to decoration its child.
#[derive(Default)]
pub struct ComposeDecorators {
  styles: HashMap<TypeId, Box<ComposeDecoratorFn>, ahash::RandomState>,
}

/// `ComposeDecorator` is a trait let you can convert your host widget to
/// another, it has same signature of `ComposeChild`, but it can be overwrote in
/// `Theme` by a function. The trait implementation only as a default logic if
/// no overwrite function in `Theme`.
pub trait ComposeDecorator: Sized {
  type Host;

  fn compose_decorator(this: State<Self>, host: Self::Host) -> Widget;
}

impl<W: ComposeDecorator + 'static> ComposeChild for W {
  type Child = W::Host;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    FnWidget::new(move |ctx: &BuildCtx| {
      let tid = TypeId::of::<W>();
      let style = ctx.find_cfg(|t| match t {
        Theme::Full(t) => t.compose_decorators.styles.get(&tid),
        Theme::Inherit(i) => i
          .compose_decorators
          .as_ref()
          .and_then(|s| s.styles.get(&tid)),
      });

      if let Some(style) = style {
        style(Box::new(this.into_writable()), Box::new(child)).build(ctx)
      } else {
        ComposeDecorator::compose_decorator(this, child).build(ctx)
      }
    })
    .into()
  }
}

impl ComposeDecorators {
  #[inline]
  pub fn override_compose_decorator<W: ComposeDecorator + 'static>(
    &mut self,
    compose_decorator: impl Fn(Stateful<W>, W::Host) -> Widget + Clone + 'static,
  ) {
    self.styles.insert(
      TypeId::of::<W>(),
      Box::new(move |this: Box<dyn Any>, host: Box<dyn Any>| {
        let this = this.downcast().unwrap_or_else(|_| {
          panic!(
            "Caller should guarantee the boxed type is Stateful<{}>.",
            type_name::<W>()
          )
        });
        let host = host.downcast().unwrap_or_else(|_| {
          panic!(
            "Caller should guarantee the boxed type is {}.",
            type_name::<W::Host>(),
          )
        });
        compose_decorator(*this, *host)
      }),
    );
  }
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, test_helper::*};
  use ribir_dev_helper::*;

  #[test]
  fn compose_decorator_smoke() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let mut theme = FullTheme::default();

    #[derive(Declare)]
    struct Size100Style;

    impl ComposeDecorator for Size100Style {
      type Host = Widget;
      fn compose_decorator(_: State<Self>, host: Self::Host) -> Widget { host }
    }
    theme
      .compose_decorators
      .override_compose_decorator::<Size100Style>(|_, host| {
        fn_widget! {
          @MockBox {
            size: Size::new(100., 100.),
            @ { host }
          }
        }
        .into()
      });

    let w = widget! {
      Size100Style { MockBox {
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
