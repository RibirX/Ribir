use std::collections::HashMap;

use crate::prelude::{Theme, Widget};

/// Compose style is a compose child widget to decoration its child.
#[derive(Clone, Default)]
pub struct ComposeStyles {
  styles: HashMap<ComposeStyleIdent, Box<dyn ComposeStyle>, ahash::RandomState>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ComposeStyleIdent(usize);

pub trait ComposeStyle: Fn(Widget) -> Widget {
  fn box_clone(&self) -> Box<dyn ComposeStyle>;

  fn as_fn(&self) -> &dyn Fn(Widget) -> Widget;
}

pub mod styles {
  use super::*;

  /// macro use to define a identify of [`ComposeStyleIdent`]!.
  #[macro_export]
  macro_rules! define_compose_style_ident {
    ($from: expr, $define: ident, $($ident: ident),+) => {
      define_compose_style_ident!($from, $define);
      define_compose_style_ident!($define, $($ident), +);
    };
    ($value: expr, $define: ident) => {
      pub const $define: ComposeStyleIdent = $value;
    }
  }

  /// macro use to specify the compose style widget for the identify.
  #[macro_export]
  macro_rules! fill_compose_style {
      ($styles: expr, $($name: path: $expr: expr),+) => {
        $($styles.set_style($name,  Box::new($expr));)+
      };
    }

  pub const BEGIN: ComposeStyleIdent = ComposeStyleIdent::new(0);
  define_compose_style_ident!(BEGIN, THEME_EXTEND);

  /// The user custom icon identify define start from.
  pub const CUSTOM_START: ComposeStyleIdent = ComposeStyleIdent::new(65536);
}

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
  fn clone(&self) -> Self { self.box_clone() }
}

impl<F: Fn(Widget) -> Widget + Clone + 'static> ComposeStyle for F {
  #[inline]
  fn box_clone(&self) -> Box<dyn ComposeStyle> { Box::new(self.clone()) }

  #[inline]
  fn as_fn(&self) -> &dyn Fn(Widget) -> Widget { self }
}
#[cfg(test)]
mod tests {
  use crate::{
    define_compose_style_ident, fill_compose_style,
    prelude::*,
    test::{expect_layout_result, ExpectRect, LayoutTestItem},
  };

  #[test]
  fn compose_style_smoke() {
    let mut theme = material::purple::light();

    define_compose_style_ident!(styles::THEME_EXTEND, SIZE_100);
    fill_compose_style!(theme.compose_styles,
      SIZE_100: |child| {
      widget! {
        SizedBox {
          size: Size::new(100., 100.),
          ExprWidget { expr: child }
        }
      }
    });

    let w = widget! {
      ExprWidget {
        theme,
        expr: SIZE_100.of(ctx),
        SizedBox {
          size: Size::zero(),
        }
      }
    };
    expect_layout_result(
      Size::new(500., 500.),
      w,
      &[LayoutTestItem {
        path: &[0],
        expect: ExpectRect {
          width: Some(100.),
          height: Some(100.),
          ..<_>::default()
        },
      }],
    )
  }
}
