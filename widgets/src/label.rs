use ribir_core::prelude::*;

#[derive(ChildOfCompose)]
pub struct Label(pub DeclareInit<CowArc<str>>);

impl Label {
  #[inline]
  pub fn new<const M: usize>(str: impl DeclareInto<CowArc<str>, M>) -> Self {
    Self(str.declare_into())
  }
}
