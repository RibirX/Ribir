use super::{
  animations::{Animate, State, Transition},
  dataflows::Dataflow,
  DeclareField,
};
use proc_macro2::Span;
use syn::Ident;

#[derive(Clone, Debug)]
pub struct FollowOn {
  pub widget: Ident,
  pub spans: Vec<Span>,
}

#[derive(Clone, Debug)]
pub struct FollowPart<'a> {
  pub origin: FollowPlace<'a>,
  pub follows: &'a [FollowOn],
}
#[derive(Clone)]
pub struct Follows<'a>(Box<[FollowPart<'a>]>);

#[derive(Clone, Copy, Debug)]
pub enum FollowPlace<'a> {
  Field(&'a DeclareField),
  DataFlow(&'a Dataflow),
  Animate(&'a Animate),
  State(&'a State),
  Transition(&'a Transition),
}

impl<'a> FollowPart<'a> {
  pub fn from_widget_field(field: &'a DeclareField) -> Option<Self> {
    field.follows.as_ref().map(|follows| Self {
      origin: FollowPlace::Field(field),
      follows: &follows,
    })
  }
}

impl<'a, IntoIter> From<IntoIter> for Follows<'a>
where
  IntoIter: IntoIterator<Item = FollowPart<'a>>,
{
  #[inline]
  fn from(iter: IntoIter) -> Self { Self(iter.into_iter().collect()) }
}

impl<'a> Follows<'a> {
  #[inline]
  pub fn from_single_part(part: FollowPart<'a>) -> Self { Self(Box::new([part])) }

  // return the iterator of tuple, the tuple compose by a field and a widget name,
  // the widget name is what the field follow on
  pub fn follow_iter(&self) -> impl Iterator<Item = (FollowPlace, &FollowOn)> {
    self
      .iter()
      .flat_map(|p| p.follows.iter().map(|f| (p.origin, f)))
  }
}

impl<'a> std::ops::Deref for Follows<'a> {
  type Target = [FollowPart<'a>];

  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<'a> std::ops::DerefMut for Follows<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

impl<'a> FromIterator<FollowPart<'a>> for Follows<'a> {
  #[inline]
  fn from_iter<T: IntoIterator<Item = FollowPart<'a>>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}
