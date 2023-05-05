use crate::prelude::*;
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

  fn compose_decorator(this: Stateful<Self>, host: Self::Host) -> Widget;
}

impl<W: ComposeDecorator + 'static> ComposeChild for W {
  type Child = W::Host;
  type Target = Widget;
  fn compose_child(this: State<Self>, child: Self::Child) -> Self::Target {
    (move |ctx: &BuildCtx| {
      let tid = TypeId::of::<W>();
      let style = ctx.find_cfg(|t| match t {
        Theme::Full(t) => t.compose_decorators.styles.get(&tid),
        Theme::Inherit(i) => i
          .compose_decorators
          .as_ref()
          .and_then(|s| s.styles.get(&tid)),
      });

      if let Some(style) = style {
        style(Box::new(this.into_writable()), Box::new(child))
      } else {
        ComposeDecorator::compose_decorator(this.into_writable(), child)
      }
    })
    .into_widget()
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
  use crate::{prelude::*, test::*};
  use std::rc::Rc;

  #[test]
  fn compose_decorator_smoke() {
    let mut theme = FullTheme::default();

    #[derive(Declare)]
    struct Size100Style;

    impl ComposeDecorator for Size100Style {
      type Host = Widget;
      fn compose_decorator(_: Stateful<Self>, host: Self::Host) -> Widget { host }
    }
    theme
      .compose_decorators
      .override_compose_decorator::<Size100Style>(|_, host| {
        widget! {
          MockBox {
            size: Size::new(100., 100.),
            DynWidget { dyns: host }
          }
        }
        .into_widget()
      });

    let w = widget! {
      Size100Style { MockBox {
        size: Size::zero(),
      }}
    };

    let ctx = AppContext {
      app_theme: Rc::new(Theme::Full(theme)),
      ..Default::default()
    };
    let mut wnd = Window::mock_window(w, Size::new(500., 500.), ctx);
    wnd.draw_frame();

    let size = layout_size_by_path(&wnd, &[0]);
    assert_eq!(size, Size::new(100., 100.));
  }
}
