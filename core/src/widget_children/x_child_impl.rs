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

impl<'p, K> XChild<'p, K> {
  pub(crate) fn from_boxed(parent: Box<dyn BoxedParent + 'p>) -> Self { Self(parent, PhantomData) }
}

impl<'p, K> RFrom<XChild<'p, K>, OtherWidget<dyn Compose>> for Widget<'p> {
  #[inline]
  fn r_from(value: XChild<'p, K>) -> Self { value.0.boxed_with_children(vec![]) }
}
