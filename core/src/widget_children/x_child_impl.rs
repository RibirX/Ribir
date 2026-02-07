use std::marker::PhantomData;

use super::*;

/// Type-erased parent wrapper with a compile-time child-arity marker.
pub struct XChild<'p, K>(pub(crate) Box<dyn BoxedParent + 'p>, PhantomData<fn() -> K>);

/// Marker for single-child parent wrappers.
pub struct SingleKind;

/// Marker for multi-child parent wrappers.
pub struct MultiKind;

/// Type-erased container enforcing single-child widget composition rules.
pub type XSingleChild<'p> = XChild<'p, SingleKind>;

/// Type-erased container for widgets that manage multiple children.
pub type XMultiChild<'p> = XChild<'p, MultiKind>;

pub(crate) mod sealed {
  use super::*;

  /// Explicit conversion into a type-erased parent wrapper.
  pub trait IntoXChild<'p, K>: Sized {
    fn into_x_child(self) -> XChild<'p, K>;
  }
}

/// Explicit conversion into a single-child parent wrapper.
pub trait IntoSingleChild<'p>: sealed::IntoXChild<'p, SingleKind> + Sized {
  #[inline]
  fn into_single_child(self) -> XSingleChild<'p> {
    <Self as sealed::IntoXChild<'p, SingleKind>>::into_x_child(self)
  }
}

impl<'p, T> IntoSingleChild<'p> for T where T: sealed::IntoXChild<'p, SingleKind> + Sized {}

/// Explicit conversion into a multi-child parent wrapper.
pub trait IntoMultiChild<'p>: sealed::IntoXChild<'p, MultiKind> + Sized {
  #[inline]
  fn into_multi_child(self) -> XMultiChild<'p> {
    <Self as sealed::IntoXChild<'p, MultiKind>>::into_x_child(self)
  }
}

impl<'p, T> IntoMultiChild<'p> for T where T: sealed::IntoXChild<'p, MultiKind> + Sized {}

impl<'p, K> XChild<'p, K> {
  pub(crate) fn from_boxed(parent: Box<dyn BoxedParent + 'p>) -> Self { Self(parent, PhantomData) }
}

impl<'p, K> sealed::IntoXChild<'p, K> for XChild<'p, K> {
  #[inline]
  fn into_x_child(self) -> XChild<'p, K> { self }
}

impl<'p, P> sealed::IntoXChild<'p, SingleKind> for P
where
  P: Parent + SingleChild + 'p,
{
  #[inline]
  fn into_x_child(self) -> XChild<'p, SingleKind> { XChild::from_boxed(Box::new(self)) }
}

impl<'p, P> sealed::IntoXChild<'p, MultiKind> for P
where
  P: Parent + MultiChild + 'p,
{
  #[inline]
  fn into_x_child(self) -> XChild<'p, MultiKind> { XChild::from_boxed(Box::new(self)) }
}

impl<'p, K> RFrom<XChild<'p, K>, OtherWidget<dyn Compose>> for Widget<'p> {
  #[inline]
  fn r_from(value: XChild<'p, K>) -> Self { value.0.boxed_with_children(vec![]) }
}
