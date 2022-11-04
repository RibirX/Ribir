use crate::prelude::*;
use std::{any::type_name, collections::HashMap};

/// Compose style is a compose child widget to decoration its child.
#[derive(Default, Clone)]
pub struct ComposeStyles {
  styles: HashMap<TypeId, Box<dyn ComposeStyleOverride>, ahash::RandomState>,
}

/// `ComposeStyle` is a trait let you can convert your host widget to another,
/// it has same signature of `ComposeChild`, but it can be overwrote in `Theme`
/// by a function. The trait implementation only as a default logic if no
/// overwrite function in `Theme`.
pub trait ComposeStyle {
  type Host;
  fn compose_style(this: Stateful<Self>, host: Self::Host) -> Widget
  where
    Self: Sized;
}

impl<W: ComposeStyle + 'static> ComposeChild for W {
  type Child = W::Host;

  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let widget = (move |ctx: &BuildCtx| {
      let style = ctx.theme().compose_styles.styles.get(&TypeId::of::<Self>());
      if let Some(style) = style {
        (style.override_fn())(Box::new(this.into_stateful()), Box::new(child))
      } else {
        ComposeStyle::compose_style(this.into_stateful(), child)
      }
    })
    .into_widget();
    ComposedWidget::<Widget, W>::new(widget).into_widget()
  }
}

impl Theme {
  #[inline]
  pub fn overwrite_compose_style<W: ComposeStyle + 'static>(
    &mut self,
    compose_style: impl Fn(Stateful<W>, W::Host) -> Widget + Clone + 'static,
  ) {
    self.compose_styles.styles.insert(
      TypeId::of::<W>(),
      Box::new(move |this: Box<dyn Any>, host: Box<dyn Any>| {
        let this = this.downcast().expect(&format!(
          "Caller should guarantee the boxed type is Stateful<{}>.",
          type_name::<W>(),
        ));
        let host = host.downcast().expect(&format!(
          "Caller should guarantee the boxed type is {}.",
          type_name::<W::Host>(),
        ));
        compose_style(*this, *host)
      }),
    );
  }
}

trait ComposeStyleOverride {
  fn box_clone(&self) -> Box<dyn ComposeStyleOverride>;

  fn override_fn(&self) -> &dyn Fn(Box<dyn Any>, Box<dyn Any>) -> Widget;
}

impl Clone for Box<dyn ComposeStyleOverride> {
  #[inline]
  fn clone(&self) -> Self { self.deref().box_clone() }
}

impl<F> ComposeStyleOverride for F
where
  F: Fn(Box<dyn Any>, Box<dyn Any>) -> Widget + Clone + 'static,
{
  #[inline]
  fn box_clone(&self) -> Box<dyn ComposeStyleOverride> { Box::new(self.clone()) }
  #[inline]
  fn override_fn(&self) -> &dyn Fn(Box<dyn Any>, Box<dyn Any>) -> Widget { self }
}

#[cfg(test)]
mod tests {
  use crate::{prelude::*, test::*};
  use std::rc::Rc;

  #[test]
  fn compose_style_smoke() {
    let mut theme = Theme::default();

    #[derive(Declare)]
    struct Size100Style;

    impl ComposeStyle for Size100Style {
      type Host = Widget;
      fn compose_style(_: Stateful<Self>, style: Self::Host) -> Widget { style }
    }
    theme.overwrite_compose_style::<Size100Style>(|_, host| {
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
      app_theme: Rc::new(theme),
      ..Default::default()
    };
    let mut wnd = Window::mock_render(w, Size::new(500., 500.), ctx);
    wnd.draw_frame();

    let rect = layout_info_by_path(&wnd, &[0]);
    assert_eq!(rect.size, Size::new(100., 100.));
  }
}
