use std::collections::hash_map::Drain;

use super::{DataFlow, DeclareField};
use proc_macro2::Span;
use syn::Ident;

#[derive(Clone)]
pub struct FollowOnVec(Box<[FollowOn]>);

#[derive(Clone, Debug)]
pub struct FollowOn {
  pub widget: Ident,
  pub spans: Vec<Span>,
}

#[derive(Clone)]
pub struct FieldFollows<'a> {
  pub field: &'a DeclareField,
  pub follows: FollowOnVec,
}

#[derive(Clone)]
pub struct DataFlowFollows<'a> {
  pub data_flow: &'a DataFlow,
  pub follows: FollowOnVec,
}

#[derive(Clone)]
pub enum WidgetFollowPart<'a> {
  Field(FieldFollows<'a>),
  DataFlow(DataFlowFollows<'a>),
}
#[derive(Clone)]
pub struct WidgetFollows<'a>(Box<[WidgetFollowPart<'a>]>);

#[derive(Clone)]
pub enum FollowOrigin<'a> {
  Field(&'a DeclareField),
  DataFlow(&'a DataFlow),
}

impl FollowOnVec {
  pub fn names(&self) -> impl Iterator<Item = &Ident> + Clone { self.iter().map(|f| &f.widget) }
}

impl<'a> FieldFollows<'a> {
  pub fn clone_from(field: &'a DeclareField) -> Option<Self> {
    field.follows.clone().map(|follows| Self { field, follows })
  }
}

impl<'a> DataFlowFollows<'a> {
  pub fn clone_from(data_flow: &'a DataFlow) -> DataFlowFollows<'a> {
    let follows = data_flow
      .from
      .follows
      .clone()
      .expect("data flow must depends on some widget");

    DataFlowFollows { data_flow, follows }
  }
}

impl<'a, IntoIter> From<IntoIter> for WidgetFollows<'a>
where
  IntoIter: IntoIterator<Item = WidgetFollowPart<'a>>,
{
  #[inline]
  fn from(iter: IntoIter) -> Self { Self(iter.into_iter().collect()) }
}

impl<'a> WidgetFollows<'a> {
  #[inline]
  pub fn from_single_part(part: WidgetFollowPart<'a>) -> Self { Self(Box::new([part])) }

  // return the iterator of tuple, the tuple compose by a field and a widget name,
  // the widget name is what the field follow on
  pub fn follow_iter(&self) -> impl Iterator<Item = (FollowOrigin, &FollowOn)> {
    self
      .iter()
      .flat_map::<Box<dyn Iterator<Item = (FollowOrigin, &FollowOn)>>, _>(|p| match p {
        WidgetFollowPart::Field(f) => Box::new(
          f.follows
            .iter()
            .map(|fo| (FollowOrigin::Field(f.field), fo)),
        ),
        WidgetFollowPart::DataFlow(d) => Box::new(
          d.follows
            .iter()
            .map(|fo| (FollowOrigin::DataFlow(d.data_flow), fo)),
        ),
      })
  }
}

impl<'a> From<Drain<'a, Ident, Vec<Span>>> for FollowOnVec {
  fn from(iter: Drain<'a, Ident, Vec<Span>>) -> Self {
    Self(
      iter
        .map(|(widget, spans)| FollowOn {
          widget,
          spans: spans.into_iter().collect(),
        })
        .collect(),
    )
  }
}

impl std::ops::Deref for FollowOnVec {
  type Target = [FollowOn];

  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl std::ops::DerefMut for FollowOnVec {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

impl<'a> std::ops::Deref for WidgetFollows<'a> {
  type Target = [WidgetFollowPart<'a>];

  fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<'a> std::ops::DerefMut for WidgetFollows<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.0 }
}

impl FromIterator<FollowOn> for FollowOnVec {
  #[inline]
  fn from_iter<T: IntoIterator<Item = FollowOn>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}

impl<'a> FromIterator<WidgetFollowPart<'a>> for WidgetFollows<'a> {
  #[inline]
  fn from_iter<T: IntoIterator<Item = WidgetFollowPart<'a>>>(iter: T) -> Self {
    Self(iter.into_iter().collect())
  }
}
