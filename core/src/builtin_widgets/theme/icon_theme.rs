// todo: replace the icon theme with named_svgs and icon fonts for improved
// functionality.
use std::collections::HashMap;

use crate::{prelude::*, render_helper::RenderProxy};

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
        let icon = Resource::new(include_crate_svg!($path, true, false));
        $theme.set_svg($name,  icon);
      )+
    };
  }

/// The user custom icon identify define start from.
pub const CUSTOM_ICON_START: NamedSvg = NamedSvg::new(65536);

/// The identify of a svg define in theme.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, ChildOfCompose)]
pub struct NamedSvg(pub usize);

impl Compose for NamedSvg {
  fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
    fn_widget! {
      pipe!(*$this).map(|v| fn_widget!{ v.of_or_miss(BuildCtx::get())})
    }
    .into_widget()
  }
}

impl IconTheme {
  pub fn new(icon_size: IconSize) -> Self {
    let svg = include_crate_svg!("src/builtin_widgets/default_named.svg", true, false);
    let miss_icon = Resource::new(svg);
    let mut icons = HashMap::<_, _, ahash::RandomState>::default();
    icons.insert(MISS_ICON, miss_icon);

    Self { icon_size, svgs: icons }
  }

  /// Retrieve the nearest `IconTheme` from the context among its ancestors
  #[inline]
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> QueryRef<Self> {
    // At least one application theme exists
    Provider::of::<Self>(ctx).unwrap()
  }

  /// Retrieve the nearest `IconTheme` from the context among its ancestors and
  /// return a write reference to the theme.
  #[inline]
  pub fn write_of(ctx: &impl AsRef<ProviderCtx>) -> WriteRef<Self> {
    // At least one application theme exists
    Provider::write_of::<Self>(ctx).unwrap()
  }

  #[inline]
  pub fn set_svg(&mut self, name: NamedSvg, icon: Resource<Svg>) -> Option<Resource<Svg>> {
    self.svgs.insert(name, icon)
  }

  #[inline]
  pub fn has_svg(&mut self, name: &NamedSvg) -> bool { self.svgs.contains_key(name) }
}

impl IconSize {
  pub fn of(ctx: &impl AsRef<ProviderCtx>) -> QueryRef<Self> {
    QueryRef::map(IconTheme::of(ctx), |i| &i.icon_size)
  }
}

impl NamedSvg {
  pub const fn new(idx: usize) -> Self { Self(idx) }

  /// get the svg icon of the ident from the context if it have otherwise return
  /// a default icon.
  pub fn of_or_miss(self, ctx: &impl AsRef<ProviderCtx>) -> Resource<Svg> {
    NamedSvg::of(self, ctx)
      .or_else(|| MISS_ICON.of(ctx))
      .expect("Neither Icon({:?}) nor 'MISS_ICON' are initialized in all `IconTheme` instances.")
  }

  /// get the svg icon of the ident from the context if it have.
  pub fn of(self, ctx: &impl AsRef<ProviderCtx>) -> Option<Resource<Svg>> {
    IconTheme::of(ctx).svgs.get(&self).cloned()
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

impl RenderProxy for Resource<Svg> {
  fn proxy(&self) -> impl Deref<Target = impl Render + ?Sized> { &**self }
}
