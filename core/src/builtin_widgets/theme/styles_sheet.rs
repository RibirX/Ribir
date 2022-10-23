pub mod icons {
  use crate::{define_icon_ident, prelude::IconIdent};

  pub const BEGIN: IconIdent = IconIdent::new(0);
  define_icon_ident!(BEGIN, CHECKED, UNCHECKED, INDETERMINATE, THEME_EXTEND);
}

pub mod cs {
  use crate::{define_compose_style_ident, prelude::ComposeStyleIdent};
  pub const BEGIN: ComposeStyleIdent = ComposeStyleIdent::new(0);

  define_compose_style_ident! {
    BEGIN,
    SCROLLBAR_TRACK,
    SCROLLBAR_THUMB,
    THEME_EXTEND
  }
}
