use ribir_core::prelude::*;

/// There are difference between `Label` and `Text`. `Text` design to use as an
/// individual widget and user can specify its style, but `Label` only can used
/// with its purpose widget, and its style is detected by its purpose widget not
/// user.
#[derive(Clone)]
pub struct Label(pub CowArc<str>);

impl Label {
  #[inline]
  pub fn new(str: impl Into<CowArc<str>>) -> Self { Label(str.into()) }
}
