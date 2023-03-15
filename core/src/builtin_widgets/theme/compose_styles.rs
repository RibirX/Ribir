use crate::prelude::*;
use std::{any::type_name, collections::HashMap};

type ComposeStyleFn = dyn Fn(Box<dyn Any>, Box<dyn Any>) -> Widget;
/// Compose style is a compose child widget to decoration its child.
#[derive(Default)]
pub struct ComposeStyles {
  styles: HashMap<TypeId, Box<ComposeStyleFn>, ahash::RandomState>,
}

/// `ComposeStyle` is a trait let you can convert your host widget to another,
/// it has same signature of `ComposeChild`, but it can be overwrote in `Theme`
/// by a function. The trait implementation only as a default logic if no
/// overwrite function in `Theme`.
pub trait ComposeStyle: Sized {
  type Host;
  fn compose_style(this: Stateful<Self>, host: Self::Host) -> Widget;
}

impl<W: ComposeStyle + 'static> ComposeChild for W {
  type Child = W::Host;

  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    (move |ctx: &BuildCtx| {
      let tid = TypeId::of::<W>();
      let style = ctx.find_cfg(|t| match t {
        Theme::Full(t) => t.compose_styles.styles.get(&tid),
        Theme::Inherit(i) => i.compose_styles.as_ref().and_then(|s| s.styles.get(&tid)),
      });

      if let Some(style) = style {
        style(Box::new(this.into_writable()), Box::new(child))
      } else {
        ComposeStyle::compose_style(this.into_writable(), child)
      }
    })
    .into_widget()
  }
}

impl ComposeStyles {
  #[inline]
  pub fn override_compose_style<W: ComposeStyle + 'static>(
    &mut self,
    compose_style: impl Fn(Stateful<W>, W::Host) -> Widget + Clone + 'static,
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
        compose_style(*this, *host)
      }),
    );
  }
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, test::*};
  use std::rc::Rc;

  #[test]
  fn compose_style_smoke() {
    let mut theme = FullTheme::default();

    #[derive(Declare)]
    struct Size100Style;

    impl ComposeStyle for Size100Style {
      type Host = Widget;
      fn compose_style(_: Stateful<Self>, style: Self::Host) -> Widget { style }
    }
    theme
      .compose_styles
      .override_compose_style::<Size100Style>(|_, host| {
        widget! {
          MockBox {
            size: Size::new(100., 100.),
            DynWidget { dyns: host }
          }
        }
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
