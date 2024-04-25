use std::collections::HashMap;

use crate::prelude::*;

/// The theme of icon, which specify the icon size standard and provide a store
/// of svg icons to use.
#[derive(Clone)]
pub struct IconTheme {
  /// icon size standard
  pub icon_size: IconSize,
  /// a collection of icons.
  svgs: HashMap<NamedSvg, Resource<Svg>, ahash::RandomState>,
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

/// default icon, use if a icon miss in `icons`.
pub const MISS_ICON: NamedSvg = NamedSvg(0);
/// The icon you can named start from.
pub const BEGIN: NamedSvg = NamedSvg::new(1);

/// macro use to define a dozen of [`SvgIdent`]! of icons.
#[macro_export]
macro_rules! define_named_svg {
    ($from: expr, $define: ident, $($ident: ident),+) => {
      define_named_svg!($from, $define);
      define_named_svg!(NamedSvg($define.0 + 1), $($ident), +);
    };
    ($value: expr, $define: ident) => {
      pub const $define: NamedSvg = $value;
    }
  }

/// macro use to specify icon of [`SvgIdent`]! in [`IconTheme`]!.
#[macro_export]
macro_rules! fill_svgs {
    ($theme: expr, $($name: path: $path: literal),+) => {
      $(
        let icon = Resource::new(include_crate_svg!($path));
        $theme.set_svg($name,  icon);
      )+
    };
  }

/// The user custom icon identify define start from.
pub const CUSTOM_ICON_START: NamedSvg = NamedSvg::new(65536);

/// The identify of a svg define in theme.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct NamedSvg(pub usize);

impl Compose for NamedSvg {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! { @ { pipe!($this.of_or_miss(ctx!())) }}
  }
}

impl IconTheme {
  pub fn new(icon_size: IconSize) -> Self {
    let svg = include_crate_svg!("./icons/miss_icon.svg");
    let miss_icon = Resource::new(svg);
    let mut icons = HashMap::<_, _, ahash::RandomState>::default();
    icons.insert(MISS_ICON, miss_icon);

    Self { icon_size, svgs: icons }
  }

  #[inline]
  pub fn set_svg(&mut self, name: NamedSvg, icon: Resource<Svg>) -> Option<Resource<Svg>> {
    self.svgs.insert(name, icon)
  }

  #[inline]
  pub fn has_svg(&mut self, name: &NamedSvg) -> bool { self.svgs.contains_key(name) }
}

impl IconSize {
  #[inline]
  pub fn of<'a>(ctx: &'a BuildCtx) -> &'a Self {
    ctx
      .find_cfg(|t| match t {
        Theme::Full(t) => Some(&t.icon_theme.icon_size),
        Theme::Inherit(i) => i.icon_size.as_ref(),
      })
      .unwrap()
  }
}

impl NamedSvg {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get the svg icon of the ident from the context if it have otherwise return
  /// a default icon.
  pub fn of_or_miss(self, ctx: &BuildCtx) -> Resource<Svg> {
    ctx
      .find_cfg(|t| match t {
        Theme::Full(t) => t.icon_theme.svgs.get(&self).or_else(|| {
          log::info!("Icon({:?})  not init in theme.", self);
          Some(t.icon_theme.svgs.get(&MISS_ICON).unwrap())
        }),
        Theme::Inherit(i) => i
          .icons
          .as_ref()
          .and_then(|icons| icons.get(&self)),
      })
      .unwrap()
      .clone()
  }

  /// get the svg icon of the ident from the context if it have.
  pub fn of(self, ctx: &BuildCtx) -> Option<Resource<Svg>> {
    ctx
      .find_cfg(|t| match t {
        Theme::Full(t) => t.icon_theme.svgs.get(&self),
        Theme::Inherit(i) => i
          .icons
          .as_ref()
          .and_then(|icons| icons.get(&self)),
      })
      .cloned()
  }
}

pub mod svgs {
  use super::*;

  define_named_svg!(
    BEGIN,
    ADD,
    ARROW_BACK,
    ARROW_DROP_DOWN,
    ARROW_FORWARD,
    CANCEL,
    CHECK_BOX,
    CHECK_BOX_OUTLINE_BLANK,
    CHEVRON_RIGHT,
    CLOSE,
    DELETE,
    DONE,
    EXPAND_MORE,
    FAVORITE,
    HOME,
    INDETERMINATE_CHECK_BOX,
    LOGIN,
    LOGOUT,
    MENU,
    MORE_HORIZ,
    MORE_VERT,
    OPEN_IN_NEW,
    SEARCH,
    SETTINGS,
    STAR,
    TEXT_CARET,
    THEME_EXTEND
  );
}
