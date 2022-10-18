use super::Theme;
use algo::ShareResource;
use painter::{Size, SvgRender};
use std::collections::HashMap;

/// The theme of icon, which specify the icon size standard and provide a store
/// of svg icons to use.
#[derive(Debug, Clone)]
pub struct IconTheme {
  /// icon size standard
  pub icon_size: IconSize,
  /// default icon if a icon not fill or miss in `icons`.
  miss_icon: ShareResource<SvgRender>,
  /// a collection of icons.
  icons: HashMap<IconIdent, ShareResource<SvgRender>, ahash::RandomState>,
}

/// A five level standard of the size of icon in application.
#[derive(Debug, Clone)]
pub struct IconSize {
  pub tiny: Size,
  pub small: Size,
  pub medium: Size,
  pub large: Size,
  pub huge: Size,
}

pub mod icons {
  use super::*;

  /// macro use to define a dozen of [`IconIdent`]! of icons.
  #[macro_export]
  macro_rules! define_icon_ident {
    ($from: ident, $define: ident, $($ident: ident),+) => {
      define_icon_ident!($from, $define);
      define_icon_ident!($define, $($ident), +);
    };
    ($value: ident, $define: ident) => {
      pub const $define: IconIdent = $value;
    }
  }

  /// macro use to specify icon of [`IconIdent`]! in [`IconTheme`]!.
  #[macro_export]
  macro_rules! fill_icon {
    ($theme: ident, $($name: path: $path: literal),+) => {
      $($theme.icon_theme.set_icon($name,  ShareResource::new(include_svg!($path)));)+
    };
  }

  pub const BEGIN: IconIdent = IconIdent::new(0);
  define_icon_ident!(BEGIN, CHECKED, UNCHECKED, INDETERMINATE, THEME_EXTEND);
  /// The user custom icon identify define start from.
  pub const CUSTOM_START: IconIdent = IconIdent::new(65536);
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct IconIdent(usize);

impl IconTheme {
  #[inline]
  pub fn new(icon_size: IconSize, miss_icon: ShareResource<SvgRender>) -> Self {
    Self {
      icon_size,
      miss_icon,
      icons: <_>::default(),
    }
  }

  #[inline]
  pub fn set_icon(
    &mut self,
    name: IconIdent,
    icon: ShareResource<SvgRender>,
  ) -> Option<ShareResource<SvgRender>> {
    self.icons.insert(name, icon)
  }
}

impl IconSize {
  #[inline]
  pub fn of<'a>(theme: &'a Theme) -> &'a Self { &theme.icon_theme.icon_size }
}

impl IconIdent {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get the svg icon of the ident from the context if it have otherwise return
  /// a default icon.
  pub fn of_or_miss(self, theme: &Theme) -> ShareResource<SvgRender> {
    self.of(theme).unwrap_or_else(|| {
      log::info!("Icon({:?})  not init in theme.", self);
      theme.icon_theme.miss_icon.clone()
    })
  }

  /// get the svg icon of the ident from the context if it have.
  pub fn of(self, theme: &Theme) -> Option<ShareResource<SvgRender>> {
    theme.icon_theme.icons.get(&self).cloned()
  }
}

impl std::ops::Add<usize> for IconIdent {
  type Output = IconIdent;

  #[inline]
  fn add(self, rhs: usize) -> Self::Output { Self(self.0 + rhs) }
}
