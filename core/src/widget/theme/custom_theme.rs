use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

use super::Theme;

/// A map can store any type of config, useful for widget which the common
/// information of theme mod not enough and need have itself theme.
#[derive(Clone, Default)]
pub struct CustomThemes {
  themes: HashMap<TypeId, Box<dyn ExtendCustomTheme>, ahash::RandomState>,
}

/// macro use to batch set custom theme.
#[macro_export]
macro_rules! fill_custom_theme {
  ($theme: ident, $($v: expr),+) => {
    $($theme.custom_themes.set_custom_theme($($v)+);)+
  };
}

pub trait CustomTheme: Clone {
  fn default_theme(theme: &Theme) -> Self;

  #[inline]
  fn custom_theme_of(theme: &Theme) -> Self
  where
    Self: Sized + 'static,
  {
    theme
      .custom_themes
      .themes
      .get(&TypeId::of::<Self>())
      .map(|c| c.as_any().downcast_ref::<Self>().unwrap().clone())
      .unwrap_or_else(|| {
        log::warn!(
          "Not set {} in theme, use its default style",
          std::any::type_name::<Self>()
        );
        Self::default_theme(theme)
      })
  }
}

trait ExtendCustomTheme: Any {
  fn box_clone(&self) -> Box<dyn ExtendCustomTheme>;

  fn as_any(&self) -> &dyn Any;
}

impl CustomThemes {
  #[inline]
  pub fn set_custom_theme<T: Clone + CustomTheme + 'static>(&mut self, v: T) {
    self.themes.insert(v.type_id(), Box::new(v));
  }
}

impl Clone for Box<dyn ExtendCustomTheme> {
  #[inline]
  fn clone(&self) -> Self { self.box_clone() }
}

impl<T: Clone + CustomTheme + 'static> ExtendCustomTheme for T {
  #[inline]
  fn box_clone(&self) -> Box<dyn ExtendCustomTheme> { Box::new(self.clone()) }

  #[inline]
  fn as_any(&self) -> &dyn Any { self }
}
