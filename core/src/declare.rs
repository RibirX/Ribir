use crate::context::BuildCtx;

pub trait Declare {
  type Builder: DeclareBuilder;
  fn declare_builder() -> Self::Builder;
}

/// widget builder use to construct a widget in  `widget!`. See the [mod level
/// document](declare) to know how to use it.
pub trait DeclareBuilder {
  type Target;
  fn build(self, ctx: &BuildCtx) -> Self::Target;
}

#[derive(Debug, PartialEq, Hash)]
pub struct DeclareStripOption<O>(O);

impl<V> From<V> for DeclareStripOption<Option<V>> {
  #[inline]
  fn from(value: V) -> Self { Self(Some(value)) }
}

impl<V> From<Option<V>> for DeclareStripOption<Option<V>> {
  #[inline]
  fn from(value: Option<V>) -> Self { Self(value) }
}

impl<V> DeclareStripOption<Option<V>> {
  #[inline]
  pub fn into_option_value(self) -> Option<V> { self.0 }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ribir_painter::{Brush, Color};

  #[test]
  fn inner_value_into() {
    assert_eq!(
      DeclareStripOption::from(Brush::from(Color::RED)),
      DeclareStripOption(Some(Brush::from(Color::RED)))
    );
  }

  #[test]
  fn option_self_can_use_with_strip() {
    assert_eq!(
      DeclareStripOption::from(Some(Brush::from(Color::RED))),
      DeclareStripOption(Some(Brush::from(Color::RED)))
    )
  }
}

/// struct help the generate code have better type hint.
#[derive(Clone)]
pub struct DeclareFieldValue<F>(F);

impl<R, F> DeclareFieldValue<F>
where
  F: FnMut() -> R,
{
  #[inline]
  pub fn new(f: F) -> Self { Self(f) }

  #[inline]
  pub fn value(&mut self) -> R { (self.0)() }
}
