use std::{
  any::{Any, TypeId},
  cell::Ref,
  collections::HashMap,
};

use crate::context::BuildCtx;

use super::Theme;

/// A map can store any type of config, useful for widget which the common
/// information of theme mod not enough and need have itself theme.
#[derive(Default)]
pub struct CustomThemes {
  themes: HashMap<TypeId, Box<dyn Any>, ahash::RandomState>,
}

/// macro use to batch set custom theme.
#[macro_export]
macro_rules! fill_custom_theme {
  ($theme: ident, $($v: expr),+) => {
    $($theme.custom_themes.set_custom_theme($($v)+);)+
  };
}

pub trait CustomTheme: Sized + 'static {
  #[inline]
  fn of<'a>(ctx: &'a BuildCtx) -> Ref<'a, Self> {
    let tid = TypeId::of::<Self>();
    let c = ctx.find_cfg(|t| match t {
      Theme::Full(t) => t.custom_themes.themes.get(&tid),
      Theme::Inherit(i) => i.custom_themes.as_ref().and_then(|c| c.themes.get(&tid)),
    });
    c.and_then(|c| Ref::filter_map(c, |c| c.downcast_ref::<Self>()).ok())
      .unwrap_or_else(|| {
        panic!(
          "The custom theme({}) is not init in theme, use it after init.",
          std::any::type_name::<Self>()
        )
      })
  }
}

impl CustomThemes {
  #[inline]
  pub fn set_custom_theme<T: CustomTheme + 'static>(&mut self, v: T) {
    self.themes.insert(v.type_id(), Box::new(v));
  }
}
