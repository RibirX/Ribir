use ribir_core::prelude::*;

/// There are difference between `Label` and `Text`. `Text` design to use as an
/// individual widget and user can specify its style, but `Label` only can used
/// with its purpose widget, and its style is detected by its purpose widget not
/// user.
pub struct Label(pub DeclareInit<CowArc<str>>);

impl Label {
  #[inline]
  pub fn new<const M: u8>(str: impl DeclareInto<CowArc<str>, M>) -> Self {
    Self(str.declare_into())
  }
}
