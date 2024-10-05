use ribir_core::prelude::*;

pub struct Label(pub DeclareInit<CowArc<str>>);

impl Label {
  #[inline]
  pub fn new<const M: u8>(str: impl DeclareInto<CowArc<str>, M>) -> Self {
    Self(str.declare_into())
  }
}
