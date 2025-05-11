use ribir_core::prelude::*;

pub struct Label(pub PipeValue<CowArc<str>>);

impl Label {
  #[inline]
  pub fn new<K: ?Sized>(str: impl RInto<PipeValue<CowArc<str>>, K>) -> Self { Self(str.r_into()) }
}
