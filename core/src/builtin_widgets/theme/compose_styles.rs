use crate::prelude::*;
use smallvec::SmallVec;
use std::collections::HashMap;

/// Compose style is a compose child widget to decoration its child.
#[derive(Clone, Default)]
pub struct ComposeStyles {
  styles: HashMap<ComposeStyleIdent, Box<dyn ComposeStyle>, ahash::RandomState>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ComposeStyleIdent(pub usize);

pub trait ComposeStyle: Fn(Widget) -> Widget {
  fn box_clone(&self) -> Box<dyn ComposeStyle>;

  fn as_fn(&self) -> &dyn Fn(Widget) -> Widget;
}

#[derive(Declare)]
pub struct ComposeStylesWidget {
  #[declare(builtin, convert=into)]
  compose_styles: SmallVec<[ComposeStyleIdent; 1]>,
}

/// macro use to define a identify of [`ComposeStyleIdent`]!.
#[macro_export]
macro_rules! define_compose_style_ident {
    ($from: expr, $define: ident, $($ident: ident),+) => {
      define_compose_style_ident!($from, $define);
      define_compose_style_ident!(ComposeStyleIdent($define.0 + 1), $($ident), +);
    };
    ($value: expr, $define: ident) => {
      pub const $define: ComposeStyleIdent = $value;
    }
  }

/// macro use to specify the compose style widget for the identify.
#[macro_export]
macro_rules! fill_compose_style {
  ($theme: expr, $($name: path: $expr: expr),+) => {
    $($theme.compose_styles.set_style($name, Box::new($expr));)+
  };
}

/// The user custom icon identify define start from.
pub const CUSTOM_START: ComposeStyleIdent = ComposeStyleIdent::new(65536);

impl ComposeStyles {
  #[inline]
  pub fn set_style(
    &mut self,
    ident: ComposeStyleIdent,
    style: Box<dyn ComposeStyle>,
  ) -> Option<Box<dyn ComposeStyle>> {
    self.styles.insert(ident, style)
  }
}

impl ComposeStyleIdent {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get the svg icon of the ident from the context if it have.
  pub fn of<'a>(self, theme: &'a Theme) -> Option<&'a dyn Fn(Widget) -> Widget> {
    theme
      .compose_styles
      .styles
      .get(&self)
      .map(ComposeStyle::as_fn)
  }
}

impl Clone for Box<dyn ComposeStyle> {
  #[inline]
  fn clone(&self) -> Self { self.deref().box_clone() }
}

impl<F: Fn(Widget) -> Widget + Clone + 'static> ComposeStyle for F {
  #[inline]
  fn box_clone(&self) -> Box<dyn ComposeStyle> { Box::new(self.clone()) }

  #[inline]
  fn as_fn(&self) -> &dyn Fn(Widget) -> Widget { self }
}

impl ComposeChild for ComposeStylesWidget {
  type Child = Widget;

  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget {
    let this = match this {
      StateWidget::Stateless(this) => this,
      StateWidget::Stateful(_) => {
        panic!(
          "compose styles not support as a stateful widget, it's not support to \
          reactive on the change. So not directly depends on any others as a \
          builtin widget declare in `widget!`, or use `ExprWidget` to generate \
          dynamic compose style instead of."
        )
      }
    };

    widget! {
      ExprWidget {
        expr: this.compose_styles.iter().filter_map(|compose_style| {
          let style = compose_style.of(ctx.theme());
          if style.is_none() {
            log::warn!("use an compose style not init in theme.");
          }
          style
        }).fold(child, | child, compose_style| {
          compose_style(child)
        })
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{define_compose_style_ident, fill_compose_style, prelude::*, test::*};
  use std::rc::Rc;

  #[test]
  fn compose_style_smoke() {
    let mut theme = material::purple::light();

    define_compose_style_ident!(cs::THEME_EXTEND, SIZE_100);
    fill_compose_style!(theme,
      SIZE_100: |child| {
      widget! {
        MockBox {
          size: Size::new(100., 100.),
          ExprWidget { expr: child }
        }
      }
    });

    let w = widget! {
      MockBox {
        compose_styles: [SIZE_100],
        size: Size::zero(),
      }
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
